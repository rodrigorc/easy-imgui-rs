/*!
 * This example downloads a big file using tokio/reqwest using different kinds of asyncs.
 * See the comments on each implementation.
 */

use easy_imgui::future::FutureHandleGuard;
use easy_imgui_window::{
    AppEvent, AppHandler, Application, Args, EventLoopExt, EventResult, FutureBackCaller,
    LocalProxy, easy_imgui as imgui, winit,
};
use imgui::{lbl, lbl_id};
use std::num::Wrapping;
use std::time::Duration;
use winit::{
    event::WindowEvent,
    event_loop::{EventLoop, EventLoopProxy},
};

fn main() {
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Info)
        .parse_default_env()
        .init();

    // We could just use `#[tokio::main]`
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let _rt_guard = rt.enter();

    let event_loop = EventLoop::with_user_event().build().unwrap();

    let mut main = AppHandler::<App>::new(&event_loop, ());
    main.attributes().title = String::from("Example");

    event_loop.run_app(&mut main).unwrap();
}

struct App {
    frame: Wrapping<u32>,
    proxy: LocalProxy<Self>,
    handle_ticking: Option<FutureHandleGuard<()>>,
    tick: Wrapping<u32>,
    download_progress_1: (Option<u64>, u64),
    download_handle_1: Option<FutureHandleGuard<()>>,
    download_progress_2: (Option<u64>, u64),
    download_handle_2: Option<tokio::task::JoinHandle<()>>,
    download_progress_3_watch: tokio::sync::watch::Sender<(Option<u64>, u64)>,
    download_handle_3: Option<tokio::task::JoinHandle<()>>,
}

impl Application for App {
    type UserEvent = ();
    type Data = ();
    fn new(args: Args<Self>) -> App {
        let proxy = args.local_proxy();

        // This idle future converts watch events into `ping_user_input()`.
        let download_progress_3_watch = tokio::sync::watch::Sender::new((None, 0));
        args.spawn_idle({
            let cb = args.future_back();
            let mut receiver = download_progress_3_watch.subscribe();
            async move {
                while let Ok(()) = receiver.changed().await {
                    cb.run(|_app, mut args| {
                        args.ping_user_input();
                    });
                }
            }
        });

        App {
            frame: Wrapping(0),
            proxy,
            handle_ticking: None,
            tick: Wrapping(0),
            download_progress_1: (None, 0),
            download_handle_1: None,
            download_progress_2: (None, 0),
            download_handle_2: None,
            download_progress_3_watch,
            download_handle_3: None,
        }
    }
    fn window_event(&mut self, args: Args<Self>, _event: WindowEvent, res: EventResult) {
        if res.window_closed {
            args.event_loop.exit();
        }
    }
}

impl imgui::UiBuilder for App {
    fn do_ui(&mut self, ui: &imgui::Ui<Self>) {
        self.frame += 1;
        //ui.set_next_window_size(vec2(400.0, 600.0), imgui::Cond::Once);
        ui.window_config(lbl(c"Idle test"))
            .open(&mut true)
            .with(|| {
                ui.text(&format!("Frame: {}", self.frame));

                ui.separator_text(c"async-std idle future");
                let mut ticking = self.handle_ticking.is_some();
                if ui.checkbox(lbl_id("Run ticks", "tick"), &mut ticking) {
                    if ticking {
                        let cb = self.proxy.future_back();
                        let handle = self.proxy.spawn_idle(async move {
                            loop {
                                async_std::task::sleep(Duration::from_millis(250)).await;
                                cb.run(|this, mut args| {
                                    this.tick += 1;
                                    // Without this ping the UI will not be updated with the last
                                    // value. The future would still run, though.
                                    args.ping_user_input();
                                });
                            }
                        });
                        self.handle_ticking = Some(handle.guard());
                    } else {
                        self.handle_ticking = None;
                    }
                }
                ui.text(&format!("Tick: {}", self.tick));

                ui.separator_text(c"Tokio idle future");
                if ui.button(lbl_id("Download", "download1")) {
                    let cb = self.proxy.future_back();
                    let handle = self.proxy.spawn_idle(async move {
                        match test_download_1(cb).await {
                            Ok(_) => {}
                            Err(e) => {
                                eprintln!("{e:#?}");
                            }
                        }
                    });
                    self.download_handle_1 = Some(handle.guard());
                }
                let progress = self
                    .download_progress_1
                    .0
                    .map(|len| self.download_progress_1.1 as f64 / len as f64)
                    .unwrap_or(0.0);
                let overlay = match self.download_progress_1.0 {
                    None => format!("{}", self.download_progress_1.1),
                    Some(len) => format!("{} / {}", self.download_progress_1.1, len),
                };
                ui.progress_bar_config(progress as f32)
                    .overlay(overlay)
                    .build();

                ui.separator_text(c"Tokio spawn future");
                if ui.button(lbl_id("Download", "download2")) {
                    if let Some(handle) = self.download_handle_2.take() {
                        handle.abort();
                    }
                    let proxy = self.proxy.event_proxy().clone();
                    let handle = tokio::spawn(async move {
                        match test_download_2(proxy).await {
                            Ok(_) => {}
                            Err(e) => {
                                eprintln!("{e:#?}");
                            }
                        }
                    });
                    self.download_handle_2 = Some(handle);
                }
                let progress = self
                    .download_progress_2
                    .0
                    .map(|len| self.download_progress_2.1 as f64 / len as f64)
                    .unwrap_or(0.0);
                let overlay = match self.download_progress_2.0 {
                    None => format!("{}", self.download_progress_2.1),
                    Some(len) => format!("{} / {}", self.download_progress_2.1, len),
                };
                ui.progress_bar_config(progress as f32)
                    .overlay(overlay)
                    .build();

                ui.separator_text(c"Tokio spawn future with watch");
                if ui.button(lbl_id("Download", "download3")) {
                    if let Some(handle) = self.download_handle_3.take() {
                        handle.abort();
                    }
                    let sender = self.download_progress_3_watch.clone();
                    let handle = tokio::spawn(async move {
                        match test_download_3(sender).await {
                            Ok(_) => {}
                            Err(e) => {
                                eprintln!("{e:#?}");
                            }
                        }
                    });
                    self.download_handle_3 = Some(handle);
                }
                let download_progress_3 = *self.download_progress_3_watch.borrow();
                let progress = download_progress_3
                    .0
                    .map(|len| download_progress_3.1 as f64 / len as f64)
                    .unwrap_or(0.0);
                let overlay = match download_progress_3.0 {
                    None => format!("{}", download_progress_3.1),
                    Some(len) => format!("{} / {}", download_progress_3.1, len),
                };
                ui.progress_bar_config(progress as f32)
                    .overlay(overlay)
                    .build();
            });
    }
}

const URL_TEST: &str = "https://link.testfile.org/500MB";

/// This example runs as an idle future.
///
/// This is slow, because the UI usually runs at 60 fps. This means that each frame eats 1/60 seconds.
/// So if you refresh the UI every chance you have you will only run 60 "wake" events per seconds,
/// that means 60 chounks per second. And given that a chunk is usually about 10 kB, this code will
/// throttle the download at 600 kB/s.
///
/// If we don't use `ping_user_input`, then when the UI is not running it will download fast, but you can
/// never be sure of when the UI needs to refresh, just moving the mouse around would force a lot of refreshes.
async fn test_download_1(cb: FutureBackCaller<App>) -> Result<(), Box<dyn std::error::Error>> {
    let mut response = reqwest::get(URL_TEST).await?;

    let len = response.content_length();
    let mut total = 0u64;
    while let Some(chunk) = response.chunk().await? {
        total += chunk.len() as u64;
        cb.run(|app, mut args| {
            app.download_progress_1 = (len, total);
            args.ping_user_input();
        });
    }
    Ok(())
}

/// This example runs as a tokio task.
///
/// It downloads fast. To update the UI it uses the event loop `proxy` to enqueue the progress status.
/// The only drawback is that since the download is much faster than the UI, a lot of messages end up
/// being redundant.
async fn test_download_2(
    proxy: EventLoopProxy<AppEvent<App>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut response = reqwest::get(URL_TEST).await?;

    let len = response.content_length();
    let mut total = 0u64;
    while let Some(chunk) = response.chunk().await? {
        total += chunk.len() as u64;
        let proxy = proxy.clone();
        let _ = proxy.run_idle(move |app, mut args| {
            app.download_progress_2 = (len, total);
            args.ping_user_input();
        });
    }
    Ok(())
}

/// This example also runs as a tokio task.
///
/// But instead of using winit events to update the UI, it uses a tokio `watch` channel.
/// This watch channel stores the progress; its receiving end lives in an idle future, that does the
/// `ping_user_input`. That idle future runs at the UI frame rate, but that is actually a plus.
async fn test_download_3(
    progress_watch: tokio::sync::watch::Sender<(Option<u64>, u64)>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut response = reqwest::get(URL_TEST).await?;

    let len = response.content_length();
    let mut total = 0u64;
    while let Some(chunk) = response.chunk().await? {
        total += chunk.len() as u64;
        let _ = progress_watch.send((len, total));
    }
    Ok(())
}
