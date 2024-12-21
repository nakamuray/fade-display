use std::ffi::OsString;

use gtk::prelude::*;
use gtk::{gdk, gio, glib};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

const G_LOG_DOMAIN: &str = "fade-display";
const APPLICATION_ID: &str = "org.u7fa9.fade-display";
const DEFAULT_TRANSITION_DURATION: u32 = 10;

fn on_startup(_app: &gtk::Application) {
    let display = gdk::Display::default().expect("can't get default display");
    let provider = gtk::CssProvider::new();
    provider.load_from_string(
        r#"
        window {
            background-color: transparent;
        }
        box.black {
            background-color: black;
        }
    "#,
    );
    gtk::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

fn on_commandline(app: &gtk::Application, command_line: &gio::ApplicationCommandLine) -> i32 {
    let options = command_line.options_dict();
    let transition_duration: u32 = options
        .lookup::<i32>("duration")
        .unwrap()
        .map(|i| i as u32)
        .unwrap_or(DEFAULT_TRANSITION_DURATION);
    let transition_duration = transition_duration * 1000;

    let mut next_command_line: Vec<_> = command_line.arguments().into_iter().skip(1).collect();
    if let Some(index) = next_command_line.iter().position(|s| *s == "--") {
        // remove first occurence of "--"
        next_command_line.remove(index);
    }
    create_window(app, transition_duration, next_command_line);
    return 0;
}

fn create_window(app: &gtk::Application, transition_duration: u32, command_line: Vec<OsString>) {
    let win = gtk::ApplicationWindow::new(app);
    win.init_layer_shell();
    win.set_keyboard_mode(KeyboardMode::Exclusive);
    win.set_layer(Layer::Overlay);
    win.set_anchor(Edge::Top, true);
    win.set_anchor(Edge::Left, true);
    win.set_anchor(Edge::Right, true);
    win.set_anchor(Edge::Bottom, true);
    win.set_exclusive_zone(-1);

    let stack = gtk::Stack::new();
    let box_ = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    let black_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    black_box.add_css_class("black");
    stack.add_child(&box_);
    stack.add_child(&black_box);
    stack.set_transition_duration(transition_duration);
    stack.set_transition_type(gtk::StackTransitionType::Crossfade);
    win.set_child(Some(&stack));

    win.connect_show(glib::clone!(
        #[weak]
        stack,
        move |_win| {
            stack.set_visible_child(&black_box);
        }
    ));

    stack.connect_transition_running_notify(glib::clone!(
        #[weak]
        app,
        move |stack| {
            if stack.is_transition_running() {
                return;
            }
            if command_line.len() > 0 {
                //dbg!(&command_line);
                let env = std::env::vars()
                    .map(|(k, v)| format!("{}={}", k, v))
                    .collect::<Vec<_>>();
                let ret = glib::spawn_async(
                    None as Option<&std::path::Path>,
                    &command_line.iter().map(|p| p.as_ref()).collect::<Vec<_>>(),
                    // XXX: doc says "None to inherit parent's" but how can I pass this?
                    &env.iter().map(|p| p.as_ref()).collect::<Vec<_>>(),
                    glib::SpawnFlags::SEARCH_PATH | glib::SpawnFlags::DO_NOT_REAP_CHILD,
                    None,
                );
                //dbg!(&ret);
                match ret {
                    Ok(pid) => {
                        //dbg!(pid);
                        let command_line = command_line.clone();
                        glib::spawn_future_local(async move {
                            let (_pid, status) = glib::child_watch_future(pid).await;
                            //dbg!(pid, status);
                            if status != 0 {
                                glib::g_warning!(
                                    G_LOG_DOMAIN,
                                    "{:?} exit with status {}",
                                    command_line.first().unwrap(),
                                    status
                                );
                            }
                            app.quit();
                        });
                    }
                    Err(err) => {
                        glib::g_warning!(G_LOG_DOMAIN, "{}", err);
                        app.quit();
                    }
                }
            } else {
                app.quit();
            }
        }
    ));

    // XXX: what should I do if following signal fired while still waiting for command_line process?
    let controller_motion = gtk::EventControllerMotion::new();
    controller_motion.connect_motion(glib::clone!(
        #[weak]
        app,
        move |_, _, _| app.quit()
    ));
    win.add_controller(controller_motion);

    let controller_key = gtk::EventControllerKey::new();
    controller_key.connect_key_pressed(glib::clone!(
        #[weak]
        app,
        #[upgrade_or]
        glib::Propagation::Proceed,
        move |_, _, _, _| {
            app.quit();
            glib::Propagation::Proceed
        }
    ));
    controller_key.connect_key_released(glib::clone!(
        #[weak]
        app,
        move |_, _, _, _| app.quit()
    ));
    win.add_controller(controller_key);

    let gesture_click = gtk::GestureClick::new();
    gesture_click.connect_pressed(glib::clone!(
        #[weak]
        app,
        move |_, _, _, _| app.quit()
    ));
    gesture_click.connect_released(glib::clone!(
        #[weak]
        app,
        move |_, _, _, _| app.quit()
    ));
    win.add_controller(gesture_click);

    win.present();
}

fn main() {
    let app = gtk::Application::builder()
        .application_id(APPLICATION_ID)
        .flags(gio::ApplicationFlags::HANDLES_COMMAND_LINE)
        .build();

    app.add_main_option(
        "duration",
        b'd'.into(),
        glib::OptionFlags::NONE,
        glib::OptionArg::Int,
        "fade effect duration (default 5 seconds)",
        Some("DURATION"),
    );
    app.set_option_context_parameter_string(Some("[COMMAND…]"));

    app.connect_startup(on_startup);
    app.connect_command_line(on_commandline);
    app.run();
}
