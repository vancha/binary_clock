// SPDX-License-Identifier: MPL-2.0

use crate::config::Config;
use crate::fl;
use chrono::Timelike;
use chrono::{DateTime, FixedOffset, Local, TimeZone};
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::iced::mouse;
use cosmic::iced::widget::canvas;
use cosmic::iced::{window::Id, Limits, Subscription};
use cosmic::iced::{Color, Rectangle, Renderer, Theme};
use cosmic::iced_winit::commands::popup::{destroy_popup, get_popup};
use cosmic::prelude::*;
use cosmic::widget;
use cosmic::widget::Canvas;
use cosmic::Element;
use futures_util::SinkExt;

const UTC_OFFSET_SECONDS: i32 = 3600;
const ROWS: u8 = 4;

#[derive(Debug)]
enum DisplayMode {
    BCD,
    BINARY,
}

// First, we define the data we need for drawing
#[derive(Debug)]
struct ClockWidget {
    mode: DisplayMode,
    current_time: DateTime<FixedOffset>,
}

impl ClockWidget {
    //@TODO: remove all the padding from this code
    fn column(&self, index: u8, number: u32, renderer: &Renderer, bounds: Rectangle) -> canvas::Frame {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        //some padding until i learn how to properly size the widget..
        let padding = 14.0;
        // This is the amount of space we have available, subtract the hardcoded padding
        let available_height = bounds.size().height - padding;
        // The radius will be the the available height divided by the number of circles times two
        let radius = available_height / (ROWS * 2) as f32;
        // Start at the very top, in the center of the available frame but add the padding
        let mut position = cosmic::iced::Point { x: (radius * 2.0) * index as f32, y: padding / 2.0};
        position.x += radius;
        // Increment said position by the radius, so that the first circle just touches the boundary rather than be on it
        position.y += radius;
        for circle_row in (0..ROWS as usize).rev() {
            let circle = canvas::Path::circle(position, radius);
            let active_color = Color::from_rgb(0.7, 0.7, 0.7);//Color::WHITE;
            let inactive_color = Color::from_rgb(0.2, 0.2, 0.2);;
            let circle_color = if number & (1 << circle_row) != 0 { active_color } else { inactive_color };
            frame.fill(&circle, circle_color);
            position.y += radius * 2.0;
        }
        frame
    }
}

impl<Message, Theme> cosmic::widget::canvas::Program<Message, Theme> for ClockWidget {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {

        let hours_tens_place    = self.column(0, self.current_time.hour() / 10, renderer, bounds);
        let hours               = self.column(1, self.current_time.hour() % 10, renderer, bounds);
        let ten_minutes         = self.column(2, self.current_time.minute() / 10, renderer, bounds);
        let minutes             = self.column(3, self.current_time.minute() % 10, renderer, bounds);
        let tenth_seconds       = self.column(4, self.current_time.second() / 10, renderer, bounds);
        let seconds             = self.column(5, self.current_time.second() % 10, renderer, bounds);

        vec![
            hours_tens_place.into_geometry(),
            hours.into_geometry(),
            ten_minutes.into_geometry(),
            minutes.into_geometry(),
            tenth_seconds.into_geometry(),
            seconds.into_geometry()
        ]
    }
}

/// The application model stores app-specific state used to describe its interface and
/// drive its logic.
#[derive(Default)]
pub struct AppModel {
    /// Application state which is managed by the COSMIC runtime.
    core: cosmic::Core,
    /// The popup id.
    popup: Option<Id>,
    /// Configuration data that persists between application runs.
    config: Config,
    /// Example row toggler.
    example_row: bool,
    current_time: DateTime<FixedOffset>,
}

/// Messages emitted by the application and its widgets.
#[derive(Debug, Clone)]
pub enum Message {
    TogglePopup,
    Tick,
    PopupClosed(Id),
    SubscriptionChannel,
    UpdateConfig(Config),
    ToggleExampleRow(bool),
}

/// Create a COSMIC application from the app model
impl cosmic::Application for AppModel {
    /// The async executor that will be used to run your application's commands.
    type Executor = cosmic::executor::Default;

    /// Data that your application receives to its init method.
    type Flags = ();

    /// Messages which the application and its widgets will emit.
    type Message = Message;

    /// Unique identifier in RDNN (reverse domain name notation) format.
    const APP_ID: &'static str = "com.github.pop-os.cosmic-app-template";

    fn core(&self) -> &cosmic::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::Core {
        &mut self.core
    }

    /// Initializes the application with any given flags and startup commands.
    fn init(
        core: cosmic::Core,
        _flags: Self::Flags,
    ) -> (Self, Task<cosmic::Action<Self::Message>>) {

        let offset = FixedOffset::east_opt(UTC_OFFSET_SECONDS).unwrap();
        let current_time = Local::now().with_timezone(&offset);
        // Construct the app model with the runtime's core.
        let app = AppModel {
            current_time,
            core,
            config: cosmic_config::Config::new(Self::APP_ID, Config::VERSION)
                .map(|context| match Config::get_entry(&context) {
                    Ok(config) => config,
                    Err((_errors, config)) => {
                        // for why in errors {
                        //     tracing::error!(%why, "error loading app config");
                        // }

                        config
                    }
                })
                .unwrap_or_default(),
            ..Default::default()
        };

        (app, Task::none())
    }

    fn on_close_requested(&self, id: Id) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }

    /// Describes the interface based on the current state of the application model.
    ///
    /// The applet's button in the panel will be drawn using the main view method.
    /// This view should emit messages to toggle the applet's popup window, which will
    /// be drawn using the `view_window` method.
    fn view(&self) -> Element<'_, Self::Message> {
        let c: Canvas<ClockWidget, Message, cosmic::Theme, cosmic::Renderer> =
            canvas::Canvas::new(ClockWidget {
                current_time: self.current_time,
                mode: DisplayMode::BCD,
            });

        cosmic::widget::Container::new(c)
            //.width(400)
            //.max_width(400)
            //.padding(5)
            .into()
        //cosmic::widget::text("abcdefgthi").into()

        /*
        cosmic::widget::list_column()
            .padding(0)       // <-- adjust padding here
            .spacing(0)
            .add(c)
            .into()
            */
    }

    /// The applet's popup window will be drawn using this view method. If there are
    /// multiple poups, you may match the id parameter to determine which popup to
    /// create a view for.
    fn view_window(&self, _id: Id) -> Element<'_, Self::Message> {
        let content_list = widget::list_column()
            //.padding(5)
            .spacing(0)
            .add(widget::settings::item(
                fl!("example-row"),
                widget::toggler(self.example_row).on_toggle(Message::ToggleExampleRow),
            ));

        self.core.applet.popup_container(content_list).into()
    }

    /// Register subscriptions for this application.
    ///
    /// Subscriptions are long-lived async tasks running in the background which
    /// emit messages to the application through a channel. They may be conditionally
    /// activated by selectively appending to the subscription batch, and will
    /// continue to execute for the duration that they remain in the batch.
    fn subscription(&self) -> Subscription<Self::Message> {
        struct MySubscription;

        Subscription::batch(vec![
            // Create a subscription which emits updates through a channel.
            Subscription::run_with_id(
                std::any::TypeId::of::<MySubscription>(),
                cosmic::iced::stream::channel(4, move |mut channel| async move {
                    _ = channel.send(Message::SubscriptionChannel).await;

                    futures_util::future::pending().await
                }),
            ),
            // Watch for application configuration changes.
            self.core()
                .watch_config::<Config>(Self::APP_ID)
                .map(|update| {

                    // for why in update.errors {
                    //     tracing::error!(?why, "app config error");
                    // }

                    Message::UpdateConfig(update.config)
                }),
            cosmic::iced::time::every(tokio::time::Duration::new(1,0)).map(|_|Message::Tick),
        ])
    }

    /// Handles messages emitted by the application and its widgets.
    ///
    /// Tasks may be returned for asynchronous execution of code in the background
    /// on the application's async runtime. The application will not exit until all
    /// tasks are finished.
    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        match message {
            Message::Tick => {
                let offset = FixedOffset::east_opt(UTC_OFFSET_SECONDS).unwrap();
                self.current_time = Local::now().with_timezone(&offset);
            }
            Message::SubscriptionChannel => {
                // For example purposes only.
            }
            Message::UpdateConfig(config) => {
                self.config = config;
            }
            Message::ToggleExampleRow(toggled) => self.example_row = toggled,
            Message::TogglePopup => {
                return if let Some(p) = self.popup.take() {
                    destroy_popup(p)
                } else {
                    let new_id = Id::unique();
                    self.popup.replace(new_id);
                    let mut popup_settings = self.core.applet.get_popup_settings(
                        self.core.main_window_id().unwrap(),
                        new_id,
                        None,
                        None,
                        None,
                    );
                    popup_settings.positioner.size_limits = Limits::NONE
                        .max_width(372.0)
                        .min_width(300.0)
                        .min_height(200.0)
                        .max_height(1080.0);
                    get_popup(popup_settings)
                }
            }
            Message::PopupClosed(id) => {
                if self.popup.as_ref() == Some(&id) {
                    self.popup = None;
                }
            }
        }
        Task::none()
    }

    fn style(&self) -> Option<cosmic::iced_runtime::Appearance> {
        Some(cosmic::applet::style())
    }
}
