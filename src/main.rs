mod converter;
mod error;
mod prefs;

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{
    Adjustment, Box as GtkBox, CheckButton, DropDown, Entry, Label, Orientation, PolicyType,
    ScrolledWindow, SpinButton,
};
use libadwaita::prelude::*;
use libadwaita::{
    Application as AdwApplication, ApplicationWindow, ColorScheme, HeaderBar, PreferencesGroup,
    ToolbarView,
};

use converter::{
    convert, format_decimal, parse_input_expression, Unit, UnitCategory, UnitSystem,
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
}

impl AppState {
    fn new(prefs: Preferences) -> Self {
        let system: UnitSystem = prefs.default_system.clone().into();
        let category = UnitCategory::Length;
        let input_unit = Unit::default_for_system(category, system);
        let output_unit = Unit::paired_unit(input_unit);
        Self {
            prefs,
            category,
            input_unit,
            output_unit,
            system,
            updating_ui: false,
        }
    }
}

struct UiHandles {
    category_dropdown: DropDown,
    input_unit_dropdown: DropDown,
    output_unit_dropdown: DropDown,
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
        .default_width(440)
        .default_height(580)
        .build();

    let toolbar_view = ToolbarView::new();
    toolbar_view.add_top_bar(&HeaderBar::new());

    let scroll = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(PolicyType::Automatic)
        .build();

    let page = GtkBox::new(Orientation::Vertical, 0);
    page.set_margin_top(6);
    page.set_margin_bottom(12);
    scroll.set_child(Some(&page));

    // --- Unit system ---
    let system_group = preferences_section("Unit system");
    let imperial = CheckButton::with_label("Imperial");
    let metric = CheckButton::with_label("Metric");
    metric.set_group(Some(&imperial));
    let system_row = GtkBox::new(Orientation::Horizontal, 24);
    system_row.set_margin_start(12);
    system_row.set_margin_end(12);
    system_row.append(&imperial);
    system_row.append(&metric);
    system_group.add(&system_row);

    let default_check =
        CheckButton::with_label("Use selected system on startup");
    default_check.set_margin_start(12);
    default_check.set_active(true);
    system_group.add(&default_check);
    page.append(&system_group);

    // --- Units ---
    let units_group = preferences_section("Units");
    let units_inner = GtkBox::new(Orientation::Vertical, 12);
    units_inner.set_margin_start(12);
    units_inner.set_margin_end(12);

    let category_dropdown = category_drop_down();
    units_inner.append(&labeled_widget("Category", &category_dropdown));

    let units_row = GtkBox::new(Orientation::Horizontal, 8);
    let input_unit_dropdown =
        DropDown::new(Some(gtk4::StringList::new(&[])), None::<&gtk4::Expression>);
    let output_unit_dropdown =
        DropDown::new(Some(gtk4::StringList::new(&[])), None::<&gtk4::Expression>);
    let swap_btn = gtk4::Button::with_label("⇄");
    swap_btn.set_tooltip_text(Some("Swap input and output units"));

    units_row.append(&labeled_widget("From", &input_unit_dropdown));
    units_row.append(&swap_btn);
    units_row.append(&labeled_widget("To", &output_unit_dropdown));
    units_inner.append(&units_row);
    units_group.add(&units_inner);
    page.append(&units_group);

    // --- Convert ---
    let convert_group = preferences_section("Convert");
    let convert_inner = GtkBox::new(Orientation::Vertical, 10);
    convert_inner.set_margin_start(12);
    convert_inner.set_margin_end(12);

    let input_entry = Entry::builder()
        .placeholder_text("Value — optional suffix (e.g. 25 mm, 60 mph, 100 kph)")
        .build();
    convert_inner.append(&labeled_widget("Input", &input_entry));

    let result_label = Label::builder()
        .label("—")
        .halign(gtk4::Align::Start)
        .selectable(true)
        .build();
    result_label.add_css_class("title-2");
    convert_inner.append(&labeled_widget("Result", &result_label));

    let error_label = Label::builder()
        .wrap(true)
        .halign(gtk4::Align::Start)
        .build();
    error_label.add_css_class("error");
    convert_inner.append(&error_label);
    convert_group.add(&convert_inner);
    page.append(&convert_group);

    // --- Settings ---
    let settings_group = preferences_section("Settings");
    let settings_inner = GtkBox::new(Orientation::Horizontal, 12);
    settings_inner.set_margin_start(12);
    settings_inner.set_margin_end(12);
    let precision_adj = Adjustment::new(
        state.borrow().prefs.decimal_places as f64,
        2.0,
        6.0,
        1.0,
        1.0,
        0.0,
    );
    let precision_spin = SpinButton::new(Some(&precision_adj), 1.0, 0);
    settings_inner.append(&Label::new(Some("Decimal places")));
    settings_inner.append(&precision_spin);
    settings_group.add(&settings_inner);
    page.append(&settings_group);

    toolbar_view.set_content(Some(&scroll));
    window.set_content(Some(&toolbar_view));

    let ui = UiHandles {
        category_dropdown: category_dropdown.clone(),
        input_unit_dropdown: input_unit_dropdown.clone(),
        output_unit_dropdown: output_unit_dropdown.clone(),
    };

    {
        let s = state.borrow();
        match s.system {
            UnitSystem::Imperial => imperial.set_active(true),
            UnitSystem::Metric => metric.set_active(true),
        }
        set_category_selection(&category_dropdown, s.category);
        refresh_unit_dropdowns(
            &input_unit_dropdown,
            &output_unit_dropdown,
            s.category,
            s.input_unit,
            s.output_unit,
        );
    }

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
            if let Err(e) = state_c.borrow().prefs.save() {
                eprintln!("Failed to save preferences: {e}");
            }
        }
    };

    let refresh_ui = {
        let state_c = state.clone();
        let ui_c = ui.clone_handles();
        move || {
            let (category, input_unit, output_unit) = {
                let s = state_c.borrow();
                (s.category, s.input_unit, s.output_unit)
            };
            state_c.borrow_mut().updating_ui = true;
            refresh_unit_dropdowns(
                &ui_c.input_unit_dropdown,
                &ui_c.output_unit_dropdown,
                category,
                input_unit,
                output_unit,
            );
            set_category_selection(&ui_c.category_dropdown, category);
            state_c.borrow_mut().updating_ui = false;
        }
    };

    let run_conversion = {
        let state_c = state.clone();
        let input_entry_c = input_entry.clone();
        let result_label_c = result_label.clone();
        let error_label_c = error_label.clone();
        let refresh_ui_c = refresh_ui.clone();
        move || {
            let text = input_entry_c.text().to_string();
            if text.trim().is_empty() {
                result_label_c.set_text("—");
                error_label_c.set_text("");
                return;
            }

            let parsed = match parse_input_expression(&text) {
                Ok(p) => p,
                Err(e) => {
                    result_label_c.set_text("—");
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
                    let suffix = to_unit.short_suffix();
                    result_label_c.set_text(&format!(
                        "{} {}",
                        format_decimal(result, places),
                        suffix
                    ));
                    error_label_c.set_text("");
                }
                Err(e) => {
                    result_label_c.set_text("—");
                    error_label_c.set_text(&e.to_string());
                }
            }
        }
    };

    imperial.connect_toggled({
        let state_c = state.clone();
        let run = run_conversion.clone();
        let save = save_default.clone();
        move |btn: &CheckButton| {
            if !btn.is_active() {
                return;
            }
            {
                let mut s = state_c.borrow_mut();
                if s.updating_ui {
                    return;
                }
                s.system = UnitSystem::Imperial;
                s.input_unit = Unit::default_for_system(s.category, s.system);
                s.output_unit = Unit::paired_unit(s.input_unit);
            }
            save();
            run();
        }
    });

    metric.connect_toggled({
        let state_c = state.clone();
        let run = run_conversion.clone();
        let save = save_default.clone();
        move |btn: &CheckButton| {
            if !btn.is_active() {
                return;
            }
            {
                let mut s = state_c.borrow_mut();
                if s.updating_ui {
                    return;
                }
                s.system = UnitSystem::Metric;
                s.input_unit = Unit::default_for_system(s.category, s.system);
                s.output_unit = Unit::paired_unit(s.input_unit);
            }
            save();
            run();
        }
    });

    default_check.connect_toggled({
        let save = save_default.clone();
        move |_| save()
    });

    category_dropdown.connect_notify_local(Some("selected"), {
        let state_c = state.clone();
        let run = run_conversion.clone();
        move |dropdown, _| {
            let idx = dropdown.selected();
            if idx == gtk4::INVALID_LIST_POSITION {
                return;
            }
            let category = UnitCategory::ALL[idx as usize];
            {
                let mut s = state_c.borrow_mut();
                s.category = category;
                s.input_unit = Unit::default_for_system(category, s.system);
                s.output_unit = Unit::paired_unit(s.input_unit);
            }
            run();
        }
    });

    input_unit_dropdown.connect_notify_local(Some("selected"), {
        let state_c = state.clone();
        let run = run_conversion.clone();
        move |dropdown, _| {
            if state_c.borrow().updating_ui {
                return;
            }
            let idx = dropdown.selected();
            if idx == gtk4::INVALID_LIST_POSITION {
                return;
            }
            let units = Unit::units_for_category(state_c.borrow().category);
            let unit = units[idx as usize];
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
        let run = run_conversion.clone();
        move |dropdown, _| {
            if state_c.borrow().updating_ui {
                return;
            }
            let idx = dropdown.selected();
            if idx == gtk4::INVALID_LIST_POSITION {
                return;
            }
            let units = Unit::units_for_category(state_c.borrow().category);
            let mut s = state_c.borrow_mut();
            s.output_unit = units[idx as usize];
            drop(s);
            run();
        }
    });

    swap_btn.connect_clicked({
        let state_c = state.clone();
        let run = run_conversion.clone();
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

    let run_on_input = run_conversion.clone();
    input_entry.connect_changed(move |_| run_on_input());

    precision_spin.connect_value_changed({
        let state_c = state.clone();
        let run = run_conversion.clone();
        move |spin| {
            let mut s = state_c.borrow_mut();
            s.prefs.decimal_places = spin.value() as u32;
            s.prefs.clamp_decimal_places();
            if let Err(e) = s.prefs.save() {
                eprintln!("Failed to save preferences: {e}");
            }
            drop(s);
            run();
        }
    });

    window.present();
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

fn preferences_section(title: &str) -> PreferencesGroup {
    let group = PreferencesGroup::new();
    group.set_title(title);
    group
}

fn labeled_widget(title: &str, child: &impl IsA<gtk4::Widget>) -> GtkBox {
    let col = GtkBox::new(Orientation::Vertical, 4);
    let label = Label::new(Some(title));
    label.add_css_class("dim-label");
    label.set_halign(gtk4::Align::Start);
    col.append(&label);
    col.append(child);
    col.set_hexpand(true);
    child.set_hexpand(true);
    col
}

fn category_drop_down() -> DropDown {
    let labels: Vec<&str> = UnitCategory::ALL.iter().map(|c| c.label()).collect();
    DropDown::from_strings(&labels)
}

fn set_category_selection(dropdown: &DropDown, category: UnitCategory) {
    let idx = UnitCategory::ALL
        .iter()
        .position(|&c| c == category)
        .unwrap_or(0) as u32;
    dropdown.set_selected(idx);
}

fn refresh_unit_dropdowns(
    input_dd: &DropDown,
    output_dd: &DropDown,
    category: UnitCategory,
    input_unit: Unit,
    output_unit: Unit,
) {
    populate_unit_dropdown(input_dd, category);
    populate_unit_dropdown(output_dd, category);
    set_dropdown_selection(input_dd, category, input_unit);
    set_dropdown_selection(output_dd, category, output_unit);
}

fn populate_unit_dropdown(dropdown: &DropDown, category: UnitCategory) {
    let units = Unit::units_for_category(category);
    let labels: Vec<&str> = units.iter().map(|u| u.label()).collect();
    let model = gtk4::StringList::new(&labels.iter().map(|s| *s).collect::<Vec<_>>());
    dropdown.set_model(Some(&model));
}

fn set_dropdown_selection(dropdown: &DropDown, category: UnitCategory, unit: Unit) {
    let units = Unit::units_for_category(category);
    if let Some(idx) = units.iter().position(|&u| u == unit) {
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
