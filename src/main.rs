mod converter;
mod error;
mod prefs;

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::gdk::{Key, ModifierType};
use gtk4::glib::Propagation;
use gtk4::prelude::*;
use gtk4::{
    Adjustment, Box as GtkBox, Button, CheckButton, DropDown, Entry, EventControllerKey, Label,
    Orientation, SpinButton,
};
use libadwaita::prelude::*;
use libadwaita::{
    Application as AdwApplication, ApplicationWindow, ColorScheme, HeaderBar, PreferencesGroup,
    PreferencesPage, PreferencesWindow, ToolbarView,
};

use converter::{
    convert, format_decimal, parse_input_expression, peek_unit_from_input, Unit, UnitCategory,
    UnitSystem,
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
    input_focused: bool,
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
            input_focused: false,
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

struct CopyTarget {
    input_entry: Entry,
    result_text: Rc<RefCell<String>>,
}

struct PrefsUi {
    window: PreferencesWindow,
    imperial: CheckButton,
    metric: CheckButton,
    precision_spin: SpinButton,
}

fn main() {
    let app = AdwApplication::builder().application_id(APP_ID).build();
    app.connect_startup(|app| configure_color_scheme(app));
    app.connect_activate(build_ui);
    app.run();
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

fn build_ui(app: &AdwApplication) {
    let prefs = Preferences::load().unwrap_or_default();
    let state = Rc::new(RefCell::new(AppState::new(prefs)));

    let window = ApplicationWindow::builder()
        .application(app)
        .title("QuickConvert")
        .default_width(380)
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

    let category_dropdown = category_drop_down();
    let input_unit_dropdown =
        DropDown::new(Some(gtk4::StringList::new(&[])), None::<&gtk4::Expression>);
    let output_unit_dropdown =
        DropDown::new(Some(gtk4::StringList::new(&[])), None::<&gtk4::Expression>);
    let swap_btn = Button::builder().label("⇄").tooltip_text("Swap units").build();

    let units_row = GtkBox::new(Orientation::Horizontal, 6);
    units_row.append(&compact_dropdown("Type", &category_dropdown));
    units_row.append(&compact_dropdown("From", &input_unit_dropdown));
    units_row.append(&swap_btn);
    units_row.append(&compact_dropdown("To", &output_unit_dropdown));
    root.append(&units_row);

    let input_entry = Entry::builder()
        .placeholder_text("Value (optional unit: 25 mm, 100 kPa, 180°)")
        .build();
    root.append(&input_entry);

    let result_label = Label::builder()
        .label("—")
        .halign(gtk4::Align::Start)
        .selectable(true)
        .build();
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
        result_text: result_text.clone(),
    };

    let ui = UiHandles {
        category_dropdown: category_dropdown.clone(),
        input_unit_dropdown: input_unit_dropdown.clone(),
        output_unit_dropdown: output_unit_dropdown.clone(),
    };

    let refresh_ui: Rc<dyn Fn()> = Rc::new({
        let state_c = state.clone();
        let ui_c = ui.clone_handles();
        let input_entry_c = input_entry.clone();
        move || {
            let text = input_entry_c.text().to_string();
            let (category, input_unit, output_unit, cat_opts, from_opts, to_opts) = {
                let mut s = state_c.borrow_mut();
                apply_input_filters(&text, &mut s);
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
            populate_category_dropdown(&ui_c.category_dropdown, &cat_opts, category);
            populate_unit_dropdown_list(&ui_c.input_unit_dropdown, &from_opts, input_unit);
            populate_unit_dropdown_list(&ui_c.output_unit_dropdown, &to_opts, output_unit);
            state_c.borrow_mut().updating_ui = false;
        }
    });

    refresh_ui();

    let run_conversion: Rc<dyn Fn()> = Rc::new({
        let state_c = state.clone();
        let input_entry_c = input_entry.clone();
        let result_label_c = result_label.clone();
        let error_label_c = error_label.clone();
        let result_text_c = result_text.clone();
        let refresh_ui_c = refresh_ui.clone();
        move || {
            let text = input_entry_c.text().to_string();
            if text.trim().is_empty() {
                result_label_c.set_text("—");
                *result_text_c.borrow_mut() = "—".to_string();
                error_label_c.set_text("");
                return;
            }

            let parsed = match parse_input_expression(&text) {
                Ok(p) => p,
                Err(e) => {
                    result_label_c.set_text("—");
                    *result_text_c.borrow_mut() = String::new();
                    error_label_c.set_text(&e.to_string());
                    return;
                }
            };

            let (from_unit, to_unit, places) = {
                let mut s = state_c.borrow_mut();
                if let Some(u) = parsed.unit {
                    s.input_unit = u;
                    s.category = u.category();
                    s.output_unit = auto_output_unit(u);
                }
                (
                    s.input_unit,
                    s.output_unit,
                    s.prefs.decimal_places,
                )
            };

            refresh_ui_c();

            match convert(parsed.value, from_unit, to_unit, places) {
                Ok(result) => {
                    let display = format!(
                        "{} {}",
                        format_decimal(result, places),
                        to_unit.short_suffix()
                    );
                    result_label_c.set_text(&display);
                    *result_text_c.borrow_mut() = display;
                    error_label_c.set_text("");
                }
                Err(e) => {
                    result_label_c.set_text("—");
                    *result_text_c.borrow_mut() = String::new();
                    error_label_c.set_text(&e.to_string());
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

    let focus_ctrl = gtk4::EventControllerFocus::new();
    focus_ctrl.connect_enter({
        let state_c = state.clone();
        let refresh_c = refresh_ui.clone();
        move |_| {
            state_c.borrow_mut().input_focused = true;
            refresh_c();
        }
    });
    focus_ctrl.connect_leave({
        let state_c = state.clone();
        let refresh_c = refresh_ui.clone();
        move |_| {
            state_c.borrow_mut().input_focused = false;
            refresh_c();
        }
    });
    input_entry.add_controller(focus_ctrl);

    input_entry.connect_changed({
        let run = run.clone();
        let refresh_c = refresh_ui.clone();
        move |_| {
            refresh_c();
            run();
        }
    });

    let prefs_ui = Rc::new(RefCell::new(None::<PrefsUi>));
    prefs_btn.connect_clicked({
        let window_c = window.clone();
        let state_c = state.clone();
        let prefs_ui_c = prefs_ui.clone();
        let run = run.clone();
        let refresh_c = refresh.clone();
        move |_| {
            let mut slot = prefs_ui_c.borrow_mut();
            if slot.is_none() {
                *slot = Some(build_preferences_window(
                    &window_c,
                    state_c.clone(),
                    run.clone(),
                    refresh_c.clone(),
                ));
            }
            if let Some(ref prefs) = *slot {
                sync_preferences_from_state(&state_c, prefs);
                prefs.window.present();
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

    window.present();
}

fn focus_and_select_entry(entry: &Entry) {
    entry.grab_focus();
    let len = entry.text().chars().count() as i32;
    entry.select_region(0, len);
}

fn attach_copy_shortcut(window: &ApplicationWindow, target: &CopyTarget) {
    let input_entry = target.input_entry.clone();
    let result_text = target.result_text.clone();

    let key_ctrl = EventControllerKey::new();
    key_ctrl.connect_key_pressed(move |_, key, _, modifiers| {
        if !modifiers.contains(ModifierType::CONTROL_MASK) {
            return Propagation::Proceed;
        }
        if key != Key::c {
            return Propagation::Proceed;
        }

        if input_entry.has_focus() {
            if let Some(selected) = entry_selected_text(&input_entry) {
                if let Some(display) = gtk4::gdk::Display::default() {
                    display.clipboard().set_text(&selected);
                }
                return Propagation::Stop;
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

fn build_preferences_window(
    parent: &ApplicationWindow,
    state: Rc<RefCell<AppState>>,
    run_conversion: Rc<dyn Fn()>,
    refresh_ui: Rc<dyn Fn()>,
) -> PrefsUi {
    let prefs_window = PreferencesWindow::builder()
        .title("Preferences")
        .transient_for(parent)
        .modal(true)
        .build();

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

    prefs_window.add(&page);

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
            if !btn.is_active() {
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
            if !btn.is_active() {
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
            let mut s = state_c.borrow_mut();
            s.prefs.decimal_places = spin.value() as u32;
            s.prefs.clamp_decimal_places();
            let _ = s.prefs.save();
            drop(s);
            run();
        }
    });

    PrefsUi {
        window: prefs_window,
        imperial,
        metric,
        precision_spin,
    }
}

fn sync_preferences_from_state(state: &Rc<RefCell<AppState>>, prefs: &PrefsUi) {
    let s = state.borrow();
    match s.system {
        UnitSystem::Imperial => prefs.imperial.set_active(true),
        UnitSystem::Metric => prefs.metric.set_active(true),
    }
    prefs.precision_spin.set_value(s.prefs.decimal_places as f64);
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

/// When the input field is focused, narrow dropdowns based on a detected unit suffix.
fn apply_input_filters(text: &str, state: &mut AppState) {
    if state.input_focused {
        if let Some(detected) = peek_unit_from_input(text) {
            let category = detected.category();
            state.category = category;
            state.input_unit = detected;
            if !state
                .to_options
                .iter()
                .any(|u| *u == state.output_unit)
                || state.output_unit.category() != category
            {
                state.output_unit = auto_output_unit(detected);
            }
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

fn populate_category_dropdown(
    dropdown: &DropDown,
    categories: &[UnitCategory],
    selected: UnitCategory,
) {
    let labels: Vec<&str> = categories.iter().map(|c| c.label()).collect();
    let model = gtk4::StringList::new(&labels.iter().map(|s| *s).collect::<Vec<_>>());
    dropdown.set_model(Some(&model));
    if let Some(idx) = categories.iter().position(|&c| c == selected) {
        dropdown.set_selected(idx as u32);
    }
}

fn populate_unit_dropdown_list(dropdown: &DropDown, units: &[Unit], selected: Unit) {
    let labels: Vec<&str> = units.iter().map(|u| u.label()).collect();
    let model = gtk4::StringList::new(&labels.iter().map(|s| *s).collect::<Vec<_>>());
    dropdown.set_model(Some(&model));
    if let Some(idx) = units.iter().position(|&u| u == selected) {
        dropdown.set_selected(idx as u32);
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
