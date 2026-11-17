mod converter;
mod error;
mod prefs;
mod shortcut;

#[cfg(target_os = "linux")]
mod tray;

use rust_decimal::Decimal;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;

use gtk4::gdk::{Key, ModifierType};
use gtk4::gio::ApplicationFlags;
use gtk4::glib::Propagation;
use gtk4::prelude::*;
use gtk4::{
    Adjustment, Box as GtkBox, Button, CheckButton, DropDown, Entry, EventControllerKey, Label,
    Orientation, SpinButton,
};
use libadwaita::prelude::*;
use libadwaita::{
    Application as AdwApplication, ApplicationWindow, ColorScheme, HeaderBar, PreferencesDialog,
    PreferencesGroup, PreferencesPage, ToolbarView,
};

use converter::{
    convert_pair, format_decimal, parse_optional_expression, peek_unit_from_input, ParsedInput,
    Unit, UnitCategory, UnitSystem,
};
use prefs::{DefaultSystem, Preferences};

const APP_ID: &str = "com.quickconvert.QuickConvert";

struct AppState {
    prefs: Preferences,
    category: UnitCategory,
    input_unit: Unit,
    output_unit: Unit,
    system: UnitSystem,
    updating_ui: bool,
    input_focus: InputFocus,
    category_options: Vec<UnitCategory>,
    from_options: Vec<Unit>,
    to_options: Vec<Unit>,
}

impl AppState {
    fn new(prefs: Preferences) -> Self {
        let system: UnitSystem = prefs.default_system.clone().into();
        let category = UnitCategory::Length;
        let input_unit = Unit::default_for_system(category, system);
        let output_unit = Unit::paired_unit(input_unit);
        let from_options = Unit::units_for_category(category).to_vec();
        let to_options = Unit::compatible_targets(output_unit);
        Self {
            prefs,
            category,
            input_unit,
            output_unit,
            system,
            updating_ui: false,
            input_focus: InputFocus::None,
            category_options: UnitCategory::ALL.to_vec(),
            from_options,
            to_options,
        }
    }
}

struct UiHandles {
    category_dropdown: DropDown,
    input_unit_dropdown: DropDown,
    output_unit_dropdown: DropDown,
}

/// Avoids rebuilding DropDown models when labels are unchanged (expensive on Windows).
struct DropdownCaches {
    category: RefCell<Option<Vec<String>>>,
    from: RefCell<Option<Vec<String>>>,
    to: RefCell<Option<Vec<String>>>,
}

impl DropdownCaches {
    fn new() -> Self {
        Self {
            category: RefCell::new(None),
            from: RefCell::new(None),
            to: RefCell::new(None),
        }
    }
}

struct Debouncer {
    source: Rc<RefCell<Option<gtk4::glib::SourceId>>>,
}

impl Debouncer {
    fn new() -> Self {
        Self {
            source: Rc::new(RefCell::new(None)),
        }
    }

    fn schedule(&self, delay_ms: u32, f: Rc<dyn Fn()>) {
        if let Some(id) = self.source.borrow_mut().take() {
            id.remove();
        }
        let source = self.source.clone();
        let id = gtk4::glib::timeout_add_local(Duration::from_millis(delay_ms as u64), move || {
            f();
            source.borrow_mut().take();
            gtk4::glib::ControlFlow::Break
        });
        *self.source.borrow_mut() = Some(id);
    }
}

struct WindowHandles {
    window: ApplicationWindow,
    input_entry: Entry,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InputFocus {
    None,
    Primary,
    Secondary,
}

struct CopyTarget {
    input_entry: Entry,
    second_entry: Entry,
    result_text: Rc<RefCell<String>>,
}

struct PrefsUi {
    dialog: PreferencesDialog,
    imperial: CheckButton,
    metric: CheckButton,
    precision_spin: SpinButton,
    minimize_to_tray: CheckButton,
    shortcut_enabled: CheckButton,
    shortcut_label: Label,
    shortcut_button: Button,
}

fn main() {
    #[cfg(windows)]
    configure_windows_env();

    let app = AdwApplication::builder()
        .application_id(APP_ID)
        .flags(ApplicationFlags::HANDLES_COMMAND_LINE)
        .build();

    let window_handles: Rc<RefCell<Option<WindowHandles>>> = Rc::new(RefCell::new(None));

    app.connect_startup(|app| {
        configure_color_scheme(app);
        configure_performance();
    });

    app.connect_command_line({
        let window_handles = window_handles.clone();
        move |app, cmdline| {
            let show = cmdline
                .arguments()
                .iter()
                .any(|arg| arg.to_string_lossy() == "--show");
            if show {
                if let Some(handles) = window_handles.borrow().as_ref() {
                    present_main_window(&handles.window, &handles.input_entry);
                    return 0;
                }
            }
            app.activate();
            0
        }
    });

    app.connect_activate({
        let window_handles = window_handles.clone();
        move |app| build_ui(app, &window_handles)
    });

    app.run();
}

/// GTK on Windows can stutter badly while dragging unfocused windows when animations are on.
/// See: https://gitlab.gnome.org/GNOME/gtk/-/issues/3922
#[cfg(windows)]
fn configure_windows_env() {
    use std::env;

    // Prefer GPU-backed rendering when available (no-op if unsupported).
    if env::var_os("GSK_RENDERER").is_none() {
        env::set_var("GSK_RENDERER", "ngl");
    }
}

fn configure_performance() {
    #[cfg(windows)]
    if let Some(settings) = gtk4::Settings::default() {
        settings.set_gtk_enable_animations(false);
        settings.set_gtk_overlay_scrolling(false);
    }
}

fn configure_color_scheme(app: &AdwApplication) {
    let manager = app.style_manager();

    if manager.system_supports_color_schemes() {
        manager.set_color_scheme(ColorScheme::Default);
    } else if let Some(settings) = gtk4::Settings::default() {
        let scheme = if settings.is_gtk_application_prefer_dark_theme() {
            ColorScheme::ForceDark
        } else {
            ColorScheme::ForceLight
        };
        manager.set_color_scheme(scheme);
    } else {
        manager.set_color_scheme(ColorScheme::Default);
    }

    if let Some(settings) = gtk4::Settings::default() {
        settings.set_gtk_application_prefer_dark_theme(false);
    }
}

fn build_ui(app: &AdwApplication, window_handles: &Rc<RefCell<Option<WindowHandles>>>) {
    if let Some(handles) = window_handles.borrow().as_ref() {
        present_main_window(&handles.window, &handles.input_entry);
        return;
    }

    let prefs = Preferences::load().unwrap_or_default();

    #[cfg(target_os = "linux")]
    {
        if shortcut::is_kde_session() {
            let _ = shortcut::apply_global_shortcut(
                prefs.global_shortcut_enabled,
                &prefs.global_shortcut,
            );
        }
    }

    let state = Rc::new(RefCell::new(AppState::new(prefs)));

    let window = ApplicationWindow::builder()
        .application(app)
        .title("QuickConvert")
        .icon_name(APP_ID)
        .default_width(480)
        .default_height(200)
        .build();

    let header = HeaderBar::new();
    let prefs_btn = Button::builder()
        .icon_name("preferences-system-symbolic")
        .tooltip_text("Preferences")
        .build();
    header.pack_end(&prefs_btn);

    let toolbar_view = ToolbarView::new();
    toolbar_view.add_top_bar(&header);

    let root = GtkBox::new(Orientation::Vertical, 10);
    root.set_margin_top(10);
    root.set_margin_bottom(10);
    root.set_margin_start(14);
    root.set_margin_end(14);

    let input_entry = Entry::builder()
        .placeholder_text("8.5\"  or  5'  or  25 mm")
        .tooltip_text("First dimension. \" is inches, ' is feet.")
        .hexpand(true)
        .build();
    let times_label = Label::new(Some("×"));
    times_label.add_css_class("dim-label");
    times_label.set_valign(gtk4::Align::Center);
    let input_entry_2 = Entry::builder()
        .placeholder_text("11\"  (optional)")
        .tooltip_text(
            "Second dimension. Leave blank for a single value. 8.5\" × 11\" is letter size.",
        )
        .hexpand(true)
        .build();
    let input_row = GtkBox::new(Orientation::Horizontal, 6);
    input_row.append(&input_entry);
    input_row.append(&times_label);
    input_row.append(&input_entry_2);
    root.append(&input_row);

    let category_dropdown = category_drop_down();
    let input_unit_dropdown =
        DropDown::new(Some(gtk4::StringList::new(&[])), None::<&gtk4::Expression>);
    let output_unit_dropdown =
        DropDown::new(Some(gtk4::StringList::new(&[])), None::<&gtk4::Expression>);
    let swap_btn = Button::builder()
        .label("⇄")
        .tooltip_text("Swap units")
        .build();

    let units_row = GtkBox::new(Orientation::Horizontal, 6);
    units_row.append(&compact_dropdown("Type", &category_dropdown));
    units_row.append(&compact_dropdown("From", &input_unit_dropdown));
    units_row.append(&swap_btn);
    units_row.append(&compact_dropdown("To", &output_unit_dropdown));
    root.append(&units_row);

    let result_label = Label::builder()
        .label("—")
        .halign(gtk4::Align::Start)
        .selectable(true)
        .build();
    #[cfg(windows)]
    result_label.add_css_class("title-2");
    #[cfg(not(windows))]
    result_label.add_css_class("title-1");
    root.append(&result_label);

    let error_label = Label::builder()
        .wrap(true)
        .halign(gtk4::Align::Start)
        .build();
    error_label.add_css_class("error");
    error_label.add_css_class("caption-heading");
    root.append(&error_label);

    toolbar_view.set_content(Some(&root));
    window.set_content(Some(&toolbar_view));

    let result_text = Rc::new(RefCell::new(String::from("—")));
    let copy_target = CopyTarget {
        input_entry: input_entry.clone(),
        second_entry: input_entry_2.clone(),
        result_text: result_text.clone(),
    };

    let ui = UiHandles {
        category_dropdown: category_dropdown.clone(),
        input_unit_dropdown: input_unit_dropdown.clone(),
        output_unit_dropdown: output_unit_dropdown.clone(),
    };
    let caches = Rc::new(DropdownCaches::new());

    let refresh_ui: Rc<dyn Fn()> = Rc::new({
        let state_c = state.clone();
        let ui_c = ui.clone_handles();
        let caches_c = caches.clone();
        let input_entry_c = input_entry.clone();
        let input_entry_2_c = input_entry_2.clone();
        move || {
            let text = input_entry_c.text().to_string();
            let text_2 = input_entry_2_c.text().to_string();
            let (category, input_unit, output_unit, cat_opts, from_opts, to_opts) = {
                let mut s = state_c.borrow_mut();
                apply_input_filters(&text, &text_2, &mut s);
                (
                    s.category,
                    s.input_unit,
                    s.output_unit,
                    s.category_options.clone(),
                    s.from_options.clone(),
                    s.to_options.clone(),
                )
            };
            state_c.borrow_mut().updating_ui = true;
            populate_category_dropdown(
                &ui_c.category_dropdown,
                &caches_c.category,
                &cat_opts,
                category,
            );
            populate_unit_dropdown_list(
                &ui_c.input_unit_dropdown,
                &caches_c.from,
                &from_opts,
                input_unit,
            );
            populate_unit_dropdown_list(
                &ui_c.output_unit_dropdown,
                &caches_c.to,
                &to_opts,
                output_unit,
            );
            state_c.borrow_mut().updating_ui = false;
        }
    });

    refresh_ui();

    let run_conversion: Rc<dyn Fn()> = Rc::new({
        let state_c = state.clone();
        let input_entry_c = input_entry.clone();
        let input_entry_2_c = input_entry_2.clone();
        let result_label_c = result_label.clone();
        let error_label_c = error_label.clone();
        let result_text_c = result_text.clone();
        let refresh_ui_c = refresh_ui.clone();
        move || {
            let text = input_entry_c.text().to_string();
            let text_2 = input_entry_2_c.text().to_string();

            let primary = match parse_optional_expression(&text) {
                Ok(value) => value,
                Err(e) => {
                    show_convert_error(&result_label_c, &error_label_c, &result_text_c, &e);
                    return;
                }
            };
            let secondary = match parse_optional_expression(&text_2) {
                Ok(value) => value,
                Err(e) => {
                    show_convert_error(&result_label_c, &error_label_c, &result_text_c, &e);
                    return;
                }
            };

            let Some((first, second)) = pair_inputs(primary, secondary) else {
                result_label_c.set_text("—");
                *result_text_c.borrow_mut() = "—".to_string();
                error_label_c.set_text("");
                return;
            };

            let (from_unit, to_unit, places, detected_unit) = {
                let mut s = state_c.borrow_mut();
                let detected = first.unit.or(second.as_ref().and_then(|dim| dim.unit));
                if let Some(detected) = detected {
                    assign_detected_unit(&mut s, detected);
                }
                (s.input_unit, s.output_unit, s.prefs.decimal_places, detected)
            };

            if detected_unit.is_some() {
                refresh_ui_c();
            }

            match convert_pair(&first, second.as_ref(), from_unit, to_unit, places) {
                Ok((result, second_result)) => {
                    let display = format_conversion(result, second_result, to_unit, places);
                    result_label_c.set_text(&display);
                    *result_text_c.borrow_mut() = display;
                    error_label_c.set_text("");
                }
                Err(e) => {
                    show_convert_error(&result_label_c, &error_label_c, &result_text_c, &e);
                }
            }
        }
    });

    attach_copy_shortcut(&window, &copy_target);

    let run = run_conversion.clone();
    let refresh = refresh_ui.clone();

    category_dropdown.connect_notify_local(Some("selected"), {
        let state_c = state.clone();
        let run = run.clone();
        move |dropdown, _| {
            if state_c.borrow().updating_ui {
                return;
            }
            let idx = dropdown.selected();
            if idx == gtk4::INVALID_LIST_POSITION {
                return;
            }
            let category = {
                let s = state_c.borrow();
                s.category_options
                    .get(idx as usize)
                    .copied()
                    .unwrap_or(s.category)
            };
            {
                let mut s = state_c.borrow_mut();
                s.category = category;
                s.input_unit = Unit::default_for_system(category, s.system);
                s.output_unit = auto_output_unit(s.input_unit);
            }
            run();
        }
    });

    input_unit_dropdown.connect_notify_local(Some("selected"), {
        let state_c = state.clone();
        let run = run.clone();
        move |dropdown, _| {
            if state_c.borrow().updating_ui {
                return;
            }
            let idx = dropdown.selected();
            if idx == gtk4::INVALID_LIST_POSITION {
                return;
            }
            let unit = {
                let s = state_c.borrow();
                s.from_options
                    .get(idx as usize)
                    .copied()
                    .unwrap_or(s.input_unit)
            };
            {
                let mut s = state_c.borrow_mut();
                s.input_unit = unit;
                s.output_unit = auto_output_unit(unit);
            }
            run();
        }
    });

    output_unit_dropdown.connect_notify_local(Some("selected"), {
        let state_c = state.clone();
        let run = run.clone();
        move |dropdown, _| {
            if state_c.borrow().updating_ui {
                return;
            }
            let idx = dropdown.selected();
            if idx == gtk4::INVALID_LIST_POSITION {
                return;
            }
            let unit = {
                let s = state_c.borrow();
                s.to_options
                    .get(idx as usize)
                    .copied()
                    .unwrap_or(s.output_unit)
            };
            let mut s = state_c.borrow_mut();
            s.output_unit = unit;
            drop(s);
            run();
        }
    });

    swap_btn.connect_clicked({
        let state_c = state.clone();
        let run = run.clone();
        move |_| {
            {
                let mut s = state_c.borrow_mut();
                let in_u = s.output_unit;
                let out_u = s.input_unit;
                s.input_unit = in_u;
                s.output_unit = out_u;
            }
            run();
        }
    });

    track_input_focus(&input_entry, InputFocus::Primary, &state, &refresh_ui);
    track_input_focus(&input_entry_2, InputFocus::Secondary, &state, &refresh_ui);

    let input_debouncer = Rc::new(Debouncer::new());
    let debounce_ms = if cfg!(windows) { 120 } else { 60 };
    let on_input_changed: Rc<dyn Fn()> = Rc::new({
        let run = run.clone();
        let refresh_c = refresh_ui.clone();
        let input_debouncer = input_debouncer.clone();
        move || {
            let work = Rc::new({
                let run = run.clone();
                let refresh_c = refresh_c.clone();
                move || {
                    refresh_c();
                    run();
                }
            });
            input_debouncer.schedule(debounce_ms, work);
        }
    });
    input_entry.connect_changed({
        let on_input_changed = on_input_changed.clone();
        move |_| on_input_changed()
    });
    input_entry_2.connect_changed({
        let on_input_changed = on_input_changed.clone();
        move |_| on_input_changed()
    });

    let prefs_ui = Rc::new(RefCell::new(None::<PrefsUi>));
    prefs_btn.connect_clicked({
        let window_c = window.clone();
        let state_c = state.clone();
        let prefs_ui_c = prefs_ui.clone();
        let run = run.clone();
        let refresh_c = refresh.clone();
        move |_| {
            if prefs_ui_c.borrow().is_none() {
                prefs_ui_c.replace(Some(build_preferences_dialog(
                    state_c.clone(),
                    run.clone(),
                    refresh_c.clone(),
                )));
            }
            if let Some(prefs) = prefs_ui_c.borrow().as_ref() {
                sync_preferences_from_state(&state_c, prefs);
                prefs.dialog.present(Some(&window_c));
            }
        }
    });

    attach_escape_minimize(&window, &state, &prefs_ui);

    window.connect_close_request({
        let window_c = window.clone();
        let state_c = state.clone();
        move |_| {
            if state_c.borrow().prefs.minimize_to_tray {
                window_c.set_visible(false);
                Propagation::Stop
            } else {
                Propagation::Proceed
            }
        }
    });

    window.connect_map({
        let input_entry_c = input_entry.clone();
        move |_| focus_and_select_entry(&input_entry_c)
    });

    gtk4::glib::idle_add_local_once({
        let input_entry_c = input_entry.clone();
        move || focus_and_select_entry(&input_entry_c)
    });

    *window_handles.borrow_mut() = Some(WindowHandles {
        window: window.clone(),
        input_entry: input_entry.clone(),
    });

    #[cfg(target_os = "linux")]
    spawn_system_tray(app, window_handles);

    window.present();
}

#[cfg(target_os = "linux")]
fn spawn_system_tray(app: &AdwApplication, window_handles: &Rc<RefCell<Option<WindowHandles>>>) {
    let (show_tx, show_rx) = mpsc::channel();
    let (quit_tx, quit_rx) = mpsc::channel();

    {
        let window_handles = window_handles.clone();
        gtk4::glib::timeout_add_local(Duration::from_millis(50), move || {
            while show_rx.try_recv().is_ok() {
                if let Some(handles) = window_handles.borrow().as_ref() {
                    present_main_window(&handles.window, &handles.input_entry);
                }
            }
            gtk4::glib::ControlFlow::Continue
        });
    }

    {
        let app = app.clone();
        gtk4::glib::timeout_add_local(Duration::from_millis(50), move || {
            while quit_rx.try_recv().is_ok() {
                app.quit();
            }
            gtk4::glib::ControlFlow::Continue
        });
    }

    let show = Arc::new(move || {
        let _ = show_tx.send(());
    });
    let quit = Arc::new(move || {
        let _ = quit_tx.send(());
    });

    std::thread::spawn(move || {
        let _handle = tray::QuickConvertTray::spawn(tray::TrayCallbacks { show, quit });
        loop {
            std::thread::park();
        }
    });
}

fn present_main_window(window: &ApplicationWindow, input_entry: &Entry) {
    window.set_visible(true);
    window.present();
    focus_and_select_entry(input_entry);
}

fn focus_and_select_entry(entry: &Entry) {
    entry.grab_focus();
    let len = entry.text().chars().count() as i32;
    entry.select_region(0, len);
}

fn attach_copy_shortcut(window: &ApplicationWindow, target: &CopyTarget) {
    let input_entry = target.input_entry.clone();
    let second_entry = target.second_entry.clone();
    let result_text = target.result_text.clone();

    let key_ctrl = EventControllerKey::new();
    key_ctrl.connect_key_pressed(move |_, key, _, modifiers| {
        if !modifiers.contains(ModifierType::CONTROL_MASK) {
            return Propagation::Proceed;
        }
        if key != Key::c {
            return Propagation::Proceed;
        }

        for entry in [&input_entry, &second_entry] {
            if entry.has_focus() {
                if let Some(selected) = entry_selected_text(entry) {
                    if let Some(display) = gtk4::gdk::Display::default() {
                        display.clipboard().set_text(&selected);
                    }
                    return Propagation::Stop;
                }
            }
        }

        let text = result_text.borrow().clone();
        if !text.is_empty() && text != "—" {
            if let Some(display) = gtk4::gdk::Display::default() {
                display.clipboard().set_text(&text);
            }
        }
        Propagation::Stop
    });
    window.add_controller(key_ctrl);
}

fn attach_escape_minimize(
    window: &ApplicationWindow,
    state: &Rc<RefCell<AppState>>,
    prefs_ui: &Rc<RefCell<Option<PrefsUi>>>,
) {
    let window_c = window.clone();
    let state_c = state.clone();
    let prefs_ui_c = prefs_ui.clone();

    let key_ctrl = EventControllerKey::new();
    key_ctrl.connect_key_pressed(move |_, key, _, _| {
        if key != Key::Escape {
            return Propagation::Proceed;
        }

        if let Some(prefs) = prefs_ui_c.borrow().as_ref() {
            if prefs.dialog.is_visible() {
                return Propagation::Proceed;
            }
        }

        if state_c.borrow().prefs.minimize_to_tray {
            window_c.set_visible(false);
            Propagation::Stop
        } else {
            Propagation::Proceed
        }
    });
    window.add_controller(key_ctrl);
}

fn entry_selected_text(entry: &Entry) -> Option<String> {
    let (start, end) = entry.selection_bounds()?;
    if start == end {
        return None;
    }
    let text: Vec<char> = entry.text().chars().collect();
    let start = start.clamp(0, text.len() as i32) as usize;
    let end = end.clamp(0, text.len() as i32) as usize;
    if start >= end {
        return None;
    }
    Some(text[start..end].iter().collect())
}

fn build_preferences_dialog(
    state: Rc<RefCell<AppState>>,
    run_conversion: Rc<dyn Fn()>,
    refresh_ui: Rc<dyn Fn()>,
) -> PrefsUi {
    let prefs_dialog = PreferencesDialog::builder().title("Preferences").build();

    let page = PreferencesPage::new();
    page.set_title("General");

    let system_group = PreferencesGroup::new();
    system_group.set_title("Unit system");

    let imperial = CheckButton::with_label("Imperial");
    let metric = CheckButton::with_label("Metric");
    metric.set_group(Some(&imperial));

    let system_row = GtkBox::new(Orientation::Horizontal, 24);
    system_row.set_margin_start(12);
    system_row.set_margin_end(12);
    system_row.append(&imperial);
    system_row.append(&metric);
    system_group.add(&system_row);

    let default_check = CheckButton::with_label("Use selected system on startup");
    default_check.set_margin_start(12);
    default_check.set_active(true);
    system_group.add(&default_check);
    page.add(&system_group);

    let display_group = PreferencesGroup::new();
    display_group.set_title("Display");

    let precision_adj = Adjustment::new(
        state.borrow().prefs.decimal_places as f64,
        2.0,
        6.0,
        1.0,
        1.0,
        0.0,
    );
    let precision_spin = SpinButton::new(Some(&precision_adj), 1.0, 0);

    let precision_row = GtkBox::new(Orientation::Horizontal, 12);
    precision_row.set_margin_start(12);
    precision_row.set_margin_end(12);
    precision_row.append(&Label::new(Some("Decimal places")));
    precision_row.append(&precision_spin);
    display_group.add(&precision_row);
    page.add(&display_group);

    let behavior_group = PreferencesGroup::new();
    behavior_group.set_title("Behavior");

    let minimize_to_tray =
        CheckButton::with_label("Minimize to system tray when closing or pressing Esc");
    minimize_to_tray.set_margin_start(12);
    minimize_to_tray.set_active(state.borrow().prefs.minimize_to_tray);
    behavior_group.add(&minimize_to_tray);
    page.add(&behavior_group);

    let shortcut_group = PreferencesGroup::new();
    shortcut_group.set_title("Global shortcut");

    let shortcut_enabled = CheckButton::with_label("Enable keyboard shortcut to show QuickConvert");
    shortcut_enabled.set_margin_start(12);
    shortcut_enabled.set_active(state.borrow().prefs.global_shortcut_enabled);
    shortcut_enabled.set_sensitive(shortcut::is_kde_session());
    shortcut_group.add(&shortcut_enabled);

    let shortcut_row = GtkBox::new(Orientation::Horizontal, 12);
    shortcut_row.set_margin_start(12);
    shortcut_row.set_margin_end(12);

    let shortcut_label = Label::new(Some(&state.borrow().prefs.global_shortcut));
    shortcut_label.set_halign(gtk4::Align::Start);
    shortcut_label.set_hexpand(true);

    let shortcut_button = Button::with_label("Change…");
    shortcut_button
        .set_sensitive(shortcut::is_kde_session() && state.borrow().prefs.global_shortcut_enabled);

    shortcut_row.append(&shortcut_label);
    shortcut_row.append(&shortcut_button);
    shortcut_group.add(&shortcut_row);

    if !shortcut::is_kde_session() {
        let hint = Label::new(Some(
            "Global shortcuts require KDE Plasma. Copy quickconvert.desktop to \
             ~/.local/share/kglobalaccel/ to register the shortcut.",
        ));
        hint.set_wrap(true);
        hint.set_margin_start(12);
        hint.set_margin_end(12);
        hint.add_css_class("dim-label");
        shortcut_group.add(&hint);
    }

    page.add(&shortcut_group);

    prefs_dialog.add(&page);

    let shortcut_capture = Rc::new(RefCell::new(false));
    let shortcut_key_ctrl = EventControllerKey::new();

    let save_default = {
        let state_c = state.clone();
        let default_check_c = default_check.clone();
        move || {
            if !default_check_c.is_active() {
                return;
            }
            let mut s = state_c.borrow_mut();
            let system: DefaultSystem = s.system.into();
            s.prefs.set_default_system(system);
            drop(s);
            let _ = state_c.borrow().prefs.save();
        }
    };

    imperial.connect_toggled({
        let state_c = state.clone();
        let run = run_conversion.clone();
        let refresh = refresh_ui.clone();
        let save = save_default.clone();
        move |btn: &CheckButton| {
            if !btn.is_active() || state_c.borrow().updating_ui {
                return;
            }
            {
                let mut s = state_c.borrow_mut();
                s.system = UnitSystem::Imperial;
                s.input_unit = Unit::default_for_system(s.category, s.system);
                s.output_unit = Unit::paired_unit(s.input_unit);
            }
            save();
            refresh();
            run();
        }
    });

    metric.connect_toggled({
        let state_c = state.clone();
        let run = run_conversion.clone();
        let refresh = refresh_ui.clone();
        let save = save_default.clone();
        move |btn: &CheckButton| {
            if !btn.is_active() || state_c.borrow().updating_ui {
                return;
            }
            {
                let mut s = state_c.borrow_mut();
                s.system = UnitSystem::Metric;
                s.input_unit = Unit::default_for_system(s.category, s.system);
                s.output_unit = Unit::paired_unit(s.input_unit);
            }
            save();
            refresh();
            run();
        }
    });

    default_check.connect_toggled({
        let save = save_default.clone();
        move |_| save()
    });

    precision_spin.connect_value_changed({
        let state_c = state.clone();
        let run = run_conversion.clone();
        move |spin| {
            if state_c.borrow().updating_ui {
                return;
            }
            let mut s = state_c.borrow_mut();
            s.prefs.decimal_places = spin.value() as u32;
            s.prefs.clamp_decimal_places();
            let _ = s.prefs.save();
            drop(s);
            run();
        }
    });

    minimize_to_tray.connect_toggled({
        let state_c = state.clone();
        move |btn| {
            if state_c.borrow().updating_ui {
                return;
            }
            let mut s = state_c.borrow_mut();
            s.prefs.minimize_to_tray = btn.is_active();
            let _ = s.prefs.save();
        }
    });

    shortcut_enabled.connect_toggled({
        let state_c = state.clone();
        let shortcut_button_c = shortcut_button.clone();
        let shortcut_label_c = shortcut_label.clone();
        move |btn| {
            if state_c.borrow().updating_ui {
                return;
            }
            let enabled = btn.is_active();
            shortcut_button_c.set_sensitive(enabled && shortcut::is_kde_session());
            let (enabled_pref, shortcut) = {
                let mut s = state_c.borrow_mut();
                s.prefs.global_shortcut_enabled = enabled;
                let shortcut = s.prefs.global_shortcut.clone();
                let _ = s.prefs.save();
                (enabled, shortcut)
            };
            let _ = shortcut::apply_global_shortcut(enabled_pref, &shortcut);
            shortcut_label_c.set_text(&shortcut);
        }
    });

    shortcut_button.connect_clicked({
        let shortcut_capture_c = shortcut_capture.clone();
        let shortcut_button_c = shortcut_button.clone();
        move |_| {
            *shortcut_capture_c.borrow_mut() = true;
            shortcut_button_c.set_label("Press keys…");
        }
    });

    shortcut_key_ctrl.connect_key_pressed({
        let state_c = state.clone();
        let shortcut_capture_c = shortcut_capture.clone();
        let shortcut_label_c = shortcut_label.clone();
        let shortcut_button_c = shortcut_button.clone();
        move |_, key, _, modifiers| {
            if !*shortcut_capture_c.borrow() {
                return Propagation::Proceed;
            }

            if key == Key::Escape {
                *shortcut_capture_c.borrow_mut() = false;
                shortcut_button_c.set_label("Change…");
                return Propagation::Stop;
            }

            if is_modifier_key(key) {
                return Propagation::Stop;
            }

            let Some(combo) = format_global_shortcut(key, modifiers) else {
                return Propagation::Stop;
            };

            *shortcut_capture_c.borrow_mut() = false;
            shortcut_button_c.set_label("Change…");
            shortcut_label_c.set_text(&combo);

            let enabled = {
                let mut s = state_c.borrow_mut();
                s.prefs.global_shortcut = combo.clone();
                let enabled = s.prefs.global_shortcut_enabled;
                let _ = s.prefs.save();
                enabled
            };
            let _ = shortcut::apply_global_shortcut(enabled, &combo);
            Propagation::Stop
        }
    });
    prefs_dialog.add_controller(shortcut_key_ctrl);

    PrefsUi {
        dialog: prefs_dialog,
        imperial,
        metric,
        precision_spin,
        minimize_to_tray,
        shortcut_enabled,
        shortcut_label,
        shortcut_button,
    }
}

fn sync_preferences_from_state(state: &Rc<RefCell<AppState>>, prefs: &PrefsUi) {
    let (system, decimal_places, minimize_to_tray, shortcut_enabled, shortcut) = {
        let s = state.borrow();
        (
            s.system,
            s.prefs.decimal_places,
            s.prefs.minimize_to_tray,
            s.prefs.global_shortcut_enabled,
            s.prefs.global_shortcut.clone(),
        )
    };

    state.borrow_mut().updating_ui = true;
    match system {
        UnitSystem::Imperial => prefs.imperial.set_active(true),
        UnitSystem::Metric => prefs.metric.set_active(true),
    }
    prefs.precision_spin.set_value(decimal_places as f64);
    prefs.minimize_to_tray.set_active(minimize_to_tray);
    prefs.shortcut_enabled.set_active(shortcut_enabled);
    prefs.shortcut_label.set_text(&shortcut);
    prefs
        .shortcut_button
        .set_sensitive(shortcut_enabled && shortcut::is_kde_session());
    state.borrow_mut().updating_ui = false;
}

impl UiHandles {
    fn clone_handles(&self) -> Self {
        Self {
            category_dropdown: self.category_dropdown.clone(),
            input_unit_dropdown: self.input_unit_dropdown.clone(),
            output_unit_dropdown: self.output_unit_dropdown.clone(),
        }
    }
}

fn compact_dropdown(title: &str, dropdown: &DropDown) -> GtkBox {
    let col = GtkBox::new(Orientation::Vertical, 2);
    let label = Label::new(Some(title));
    label.add_css_class("caption-heading");
    label.set_halign(gtk4::Align::Start);
    col.append(&label);
    col.append(dropdown);
    col.set_hexpand(true);
    dropdown.set_hexpand(true);
    col
}

fn category_drop_down() -> DropDown {
    let labels: Vec<&str> = UnitCategory::ALL.iter().map(|c| c.label()).collect();
    DropDown::from_strings(&labels)
}

/// When an input field is focused, narrow dropdowns based on a detected unit suffix.
fn apply_input_filters(primary: &str, secondary: &str, state: &mut AppState) {
    if state.input_focus != InputFocus::None {
        let (focused, other) = match state.input_focus {
            InputFocus::Secondary => (secondary, primary),
            InputFocus::Primary | InputFocus::None => (primary, secondary),
        };
        let detected = peek_unit_from_input(focused).or_else(|| peek_unit_from_input(other));
        if let Some(detected) = detected {
            assign_detected_unit(state, detected);
            let category = detected.category();
            state.category_options = vec![category];
            state.from_options = Unit::units_for_category(category).to_vec();
            state.to_options = Unit::compatible_targets(detected);
            return;
        }
    }

    state.category_options = UnitCategory::ALL.to_vec();
    state.from_options = Unit::units_for_category(state.category).to_vec();
    state.to_options = Unit::compatible_targets(state.input_unit);

    if !state.from_options.iter().any(|u| *u == state.input_unit) {
        state.input_unit = Unit::default_for_system(state.category, state.system);
    }
    if !state.to_options.iter().any(|u| *u == state.output_unit) {
        state.output_unit = auto_output_unit(state.input_unit);
    }
}

fn update_dropdown_model(
    dropdown: &DropDown,
    cache: &RefCell<Option<Vec<String>>>,
    labels: &[String],
    selected_idx: Option<u32>,
) {
    let needs_rebuild = cache.borrow().as_ref().map(|c| c.as_slice()) != Some(labels);
    if needs_rebuild {
        let refs: Vec<&str> = labels.iter().map(String::as_str).collect();
        let model = gtk4::StringList::new(&refs);
        dropdown.set_model(Some(&model));
        *cache.borrow_mut() = Some(labels.to_vec());
    }
    if let Some(idx) = selected_idx {
        dropdown.set_selected(idx);
    }
}

fn populate_category_dropdown(
    dropdown: &DropDown,
    cache: &RefCell<Option<Vec<String>>>,
    categories: &[UnitCategory],
    selected: UnitCategory,
) {
    let labels: Vec<String> = categories.iter().map(|c| c.label().to_string()).collect();
    let idx = categories
        .iter()
        .position(|&c| c == selected)
        .map(|i| i as u32);
    update_dropdown_model(dropdown, cache, &labels, idx);
}

fn populate_unit_dropdown_list(
    dropdown: &DropDown,
    cache: &RefCell<Option<Vec<String>>>,
    units: &[Unit],
    selected: Unit,
) {
    let labels: Vec<String> = units.iter().map(|u| u.label().to_string()).collect();
    let idx = units.iter().position(|&u| u == selected).map(|i| i as u32);
    update_dropdown_model(dropdown, cache, &labels, idx);
}

fn track_input_focus(
    entry: &Entry,
    which: InputFocus,
    state: &Rc<RefCell<AppState>>,
    refresh: &Rc<dyn Fn()>,
) {
    let focus_ctrl = gtk4::EventControllerFocus::new();
    focus_ctrl.connect_enter({
        let state_c = state.clone();
        let refresh_c = refresh.clone();
        move |_| {
            state_c.borrow_mut().input_focus = which;
            refresh_c();
        }
    });
    focus_ctrl.connect_leave({
        let state_c = state.clone();
        let refresh_c = refresh.clone();
        move |_| {
            let mut s = state_c.borrow_mut();
            if s.input_focus == which {
                s.input_focus = InputFocus::None;
            }
            drop(s);
            refresh_c();
        }
    });
    entry.add_controller(focus_ctrl);
}

fn pair_inputs(
    primary: Option<ParsedInput>,
    secondary: Option<ParsedInput>,
) -> Option<(ParsedInput, Option<ParsedInput>)> {
    match (primary, secondary) {
        (None, None) => None,
        (Some(first), second) => Some((first, second)),
        (None, Some(second)) => Some((second, None)),
    }
}

fn show_convert_error(
    result_label: &Label,
    error_label: &Label,
    result_text: &Rc<RefCell<String>>,
    error: &impl std::fmt::Display,
) {
    result_label.set_text("—");
    *result_text.borrow_mut() = String::new();
    error_label.set_text(&error.to_string());
}

fn format_conversion(first: Decimal, second: Option<Decimal>, unit: Unit, places: u32) -> String {
    match second {
        Some(second) => format!(
            "{} × {} {}",
            format_decimal(first, places),
            format_decimal(second, places),
            unit.short_suffix()
        ),
        None => format!("{} {}", format_decimal(first, places), unit.short_suffix()),
    }
}

/// Keep a target unit the user already picked. A typed suffix still switches
/// the source unit, and still picks a partner unit when the target is just
/// the automatic opposite of the previous source.
fn assign_detected_unit(state: &mut AppState, detected: Unit) {
    let keep_output = state.output_unit != Unit::paired_unit(state.input_unit)
        && state.output_unit.category() == detected.category();
    state.category = detected.category();
    state.input_unit = detected;
    if !keep_output {
        state.output_unit = auto_output_unit(detected);
    }
}

fn auto_output_unit(input: Unit) -> Unit {
    let paired = Unit::paired_unit(input);
    if paired.system() != input.system() {
        return paired;
    }
    Unit::units_for_category(input.category())
        .iter()
        .copied()
        .find(|u| u.system() != input.system())
        .unwrap_or(paired)
}

fn is_modifier_key(key: Key) -> bool {
    matches!(
        key,
        Key::Shift_L
            | Key::Shift_R
            | Key::Control_L
            | Key::Control_R
            | Key::Alt_L
            | Key::Alt_R
            | Key::Meta_L
            | Key::Meta_R
            | Key::Super_L
            | Key::Super_R
            | Key::Hyper_L
            | Key::Hyper_R
    )
}

fn format_global_shortcut(key: Key, modifiers: ModifierType) -> Option<String> {
    let key_name = key.name()?.to_string();
    if key_name.is_empty() || key_name == "VoidSymbol" {
        return None;
    }

    let mut parts = Vec::new();
    if modifiers.contains(ModifierType::CONTROL_MASK) {
        parts.push("Ctrl");
    }
    if modifiers.contains(ModifierType::SHIFT_MASK) {
        parts.push("Shift");
    }
    if modifiers.contains(ModifierType::ALT_MASK) {
        parts.push("Alt");
    }
    if modifiers.contains(ModifierType::META_MASK) {
        parts.push("Meta");
    }

    if parts.is_empty() {
        return None;
    }

    parts.push(key_name.as_str());
    Some(parts.join("+"))
}
