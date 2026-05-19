mod converter;
mod error;
mod prefs;

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{
    Adjustment, Box as GtkBox, CheckButton, DropDown, Label, Orientation, SpinButton, TextBuffer,
    TextView,
};
use libadwaita::{Application as AdwApplication, ApplicationWindow, HeaderBar, StyleManager};

use converter::{convert, format_decimal, parse_input, Unit, UnitCategory, UnitSystem};
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

fn main() {
    let app = AdwApplication::builder().application_id(APP_ID).build();
    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &AdwApplication) {
    let prefs = Preferences::load().unwrap_or_default();
    let state = Rc::new(RefCell::new(AppState::new(prefs)));

    let window = ApplicationWindow::builder()
        .application(app)
        .title("QuickConvert")
        .default_width(480)
        .default_height(520)
        .build();

    let style = StyleManager::default();
    style.set_color_scheme(libadwaita::ColorScheme::Default);

    let header = HeaderBar::new();
    window.set_titlebar(Some(&header));

    let root = GtkBox::new(Orientation::Vertical, 12);
    root.set_margin_top(12);
    root.set_margin_bottom(12);
    root.set_margin_start(12);
    root.set_margin_end(12);
    window.set_child(Some(&root));

    // --- System selection ---
    let system_box = GtkBox::new(Orientation::Horizontal, 24);
    root.append(&system_box);

    let imperial_radio = CheckButton::with_label("Imperial");
    let metric_radio = CheckButton::with_label("Metric");
    metric_radio.set_group(Some(&imperial_radio));
    let imperial_default = CheckButton::with_label("Default");
    let metric_default = CheckButton::with_label("Default");

    let imperial_row = GtkBox::new(Orientation::Horizontal, 8);
    imperial_row.append(&imperial_radio);
    imperial_row.append(&imperial_default);
    let metric_row = GtkBox::new(Orientation::Horizontal, 8);
    metric_row.append(&metric_radio);
    metric_row.append(&metric_default);
    system_box.append(&imperial_row);
    system_box.append(&metric_row);

    // --- Category ---
    let category_label = Label::new(Some("Unit type"));
    category_label.set_halign(gtk4::Align::Start);
    root.append(&category_label);
    let category_dropdown = category_drop_down();
    root.append(&category_dropdown);

    // --- Units ---
    let units_box = GtkBox::new(Orientation::Horizontal, 12);
    root.append(&units_box);

    let input_col = GtkBox::new(Orientation::Vertical, 6);
    input_col.append(&Label::new(Some("Input unit")));
    let input_unit_dropdown =
        DropDown::new(Some(gtk4::StringList::new(&[])), None::<&gtk4::Expression>);
    input_col.append(&input_unit_dropdown);
    units_box.append(&input_col);

    let output_col = GtkBox::new(Orientation::Vertical, 6);
    output_col.append(&Label::new(Some("Output unit")));
    let output_unit_dropdown =
        DropDown::new(Some(gtk4::StringList::new(&[])), None::<&gtk4::Expression>);
    output_col.append(&output_unit_dropdown);
    units_box.append(&output_col);

    let swap_btn = gtk4::Button::with_label("Swap Units");
    root.append(&swap_btn);

    // --- Input / output ---
    root.append(&Label::new(Some("Value")));
    let input_buffer = TextBuffer::new(None);
    let input_view = TextView::builder()
        .buffer(&input_buffer)
        .wrap_mode(gtk4::WrapMode::Word)
        .height_request(36)
        .build();
    root.append(&input_view);

    root.append(&Label::new(Some("Result")));
    let output_buffer = TextBuffer::new(None);
    output_buffer.set_text("—");
    let output_view = TextView::builder()
        .buffer(&output_buffer)
        .editable(false)
        .cursor_visible(false)
        .wrap_mode(gtk4::WrapMode::Word)
        .height_request(36)
        .build();
    root.append(&output_view);

    let error_label = Label::new(None);
    error_label.add_css_class("error");
    error_label.set_wrap(true);
    root.append(&error_label);

    // --- Precision ---
    let precision_box = GtkBox::new(Orientation::Horizontal, 8);
    precision_box.append(&Label::new(Some("Decimal places")));
    let precision_adj = Adjustment::new(
        state.borrow().prefs.decimal_places as f64,
        2.0,
        6.0,
        1.0,
        1.0,
        0.0,
    );
    let precision_spin = SpinButton::new(Some(&precision_adj), 1.0, 0);
    precision_box.append(&precision_spin);
    root.append(&precision_box);

    // Initial UI from state
    {
        let s = state.borrow();
        match s.system {
            UnitSystem::Imperial => imperial_radio.set_active(true),
            UnitSystem::Metric => metric_radio.set_active(true),
        }
        imperial_default.set_active(s.prefs.default_imperial);
        metric_default.set_active(s.prefs.default_metric);
        set_category_selection(&category_dropdown, s.category);
        refresh_unit_dropdowns(
            &input_unit_dropdown,
            &output_unit_dropdown,
            s.category,
            s.input_unit,
            s.output_unit,
        );
    }

    fn save_prefs(state: &Rc<RefCell<AppState>>) {
        if let Err(e) = state.borrow().prefs.save() {
            eprintln!("Failed to save preferences: {e}");
        }
    }

    fn run_conversion(
        state: &Rc<RefCell<AppState>>,
        input_buffer: &TextBuffer,
        output_buffer: &TextBuffer,
        error_label: &Label,
    ) {
        let s = state.borrow();
        let raw = input_buffer.text(&input_buffer.start_iter(), &input_buffer.end_iter(), true);
        let text = raw.as_str().trim();
        if text.is_empty() {
            output_buffer.set_text("—");
            error_label.set_text("");
            return;
        }

        match parse_input(text) {
            Ok(value) => match convert(
                value,
                s.input_unit,
                s.output_unit,
                s.prefs.decimal_places,
            ) {
                Ok(result) => {
                    output_buffer.set_text(&format_decimal(result, s.prefs.decimal_places));
                    error_label.set_text("");
                }
                Err(e) => {
                    output_buffer.set_text("—");
                    error_label.set_text(&e.to_string());
                }
            },
            Err(e) => {
                output_buffer.set_text("—");
                error_label.set_text(&e.to_string());
            }
        }
    }

    // --- Signals ---
    {
        let state_c = state.clone();
        let _category_dropdown_c = category_dropdown.clone();
        let input_unit_dropdown_c = input_unit_dropdown.clone();
        let output_unit_dropdown_c = output_unit_dropdown.clone();
        let output_buffer_c = output_buffer.clone();
        let error_label_c = error_label.clone();
        let input_buffer_c = input_buffer.clone();

        imperial_radio.connect_toggled({
            let state_c = state_c.clone();
            let input_unit_dropdown_c = input_unit_dropdown_c.clone();
            let output_unit_dropdown_c = output_unit_dropdown_c.clone();
            let output_buffer_c = output_buffer_c.clone();
            let error_label_c = error_label_c.clone();
            let input_buffer_c = input_buffer_c.clone();
            move |btn: &CheckButton| {
                if !btn.is_active() {
                    return;
                }
                let mut s = state_c.borrow_mut();
                if s.updating_ui {
                    return;
                }
                s.system = UnitSystem::Imperial;
                s.input_unit = Unit::default_for_system(s.category, s.system);
                s.output_unit = Unit::paired_unit(s.input_unit);
                drop(s);
                refresh_unit_dropdowns(
                    &input_unit_dropdown_c,
                    &output_unit_dropdown_c,
                    state_c.borrow().category,
                    state_c.borrow().input_unit,
                    state_c.borrow().output_unit,
                );
                run_conversion(
                    &state_c,
                    &input_buffer_c,
                    &output_buffer_c,
                    &error_label_c,
                );
            }
        });

        metric_radio.connect_toggled({
            let state_c = state_c.clone();
            let input_unit_dropdown_c = input_unit_dropdown_c.clone();
            let output_unit_dropdown_c = output_unit_dropdown_c.clone();
            let output_buffer_c = output_buffer_c.clone();
            let error_label_c = error_label_c.clone();
            let input_buffer_c = input_buffer_c.clone();
            move |btn: &CheckButton| {
                if !btn.is_active() {
                    return;
                }
                let mut s = state_c.borrow_mut();
                if s.updating_ui {
                    return;
                }
                s.system = UnitSystem::Metric;
                s.input_unit = Unit::default_for_system(s.category, s.system);
                s.output_unit = Unit::paired_unit(s.input_unit);
                drop(s);
                refresh_unit_dropdowns(
                    &input_unit_dropdown_c,
                    &output_unit_dropdown_c,
                    state_c.borrow().category,
                    state_c.borrow().input_unit,
                    state_c.borrow().output_unit,
                );
                run_conversion(
                    &state_c,
                    &input_buffer_c,
                    &output_buffer_c,
                    &error_label_c,
                );
            }
        });

        imperial_default.connect_toggled({
            let state_c = state_c.clone();
            let metric_default_c = metric_default.clone();
            let imperial_radio_c = imperial_radio.clone();
            move |cb| {
                if !cb.is_active() {
                    return;
                }
                metric_default_c.set_active(false);
                let mut s = state_c.borrow_mut();
                s.prefs.set_default_system(DefaultSystem::Imperial);
                imperial_radio_c.set_active(true);
                drop(s);
                save_prefs(&state_c);
            }
        });

        metric_default.connect_toggled({
            let state_c = state_c.clone();
            let imperial_default_c = imperial_default.clone();
            let metric_radio_c = metric_radio.clone();
            move |cb| {
                if !cb.is_active() {
                    return;
                }
                imperial_default_c.set_active(false);
                let mut s = state_c.borrow_mut();
                s.prefs.set_default_system(DefaultSystem::Metric);
                metric_radio_c.set_active(true);
                drop(s);
                save_prefs(&state_c);
            }
        });

        category_dropdown.connect_notify_local(Some("selected"), {
            let state_c = state_c.clone();
            let input_unit_dropdown_c = input_unit_dropdown_c.clone();
            let output_unit_dropdown_c = output_unit_dropdown_c.clone();
            let output_buffer_c = output_buffer_c.clone();
            let error_label_c = error_label_c.clone();
            let input_buffer_c = input_buffer_c.clone();
            move |dropdown, _| {
                let idx = dropdown.selected();
                if idx == gtk4::INVALID_LIST_POSITION {
                    return;
                }
                let category = UnitCategory::ALL[idx as usize];
                let mut s = state_c.borrow_mut();
                s.category = category;
                s.input_unit = Unit::default_for_system(category, s.system);
                s.output_unit = Unit::paired_unit(s.input_unit);
                drop(s);
                refresh_unit_dropdowns(
                    &input_unit_dropdown_c,
                    &output_unit_dropdown_c,
                    state_c.borrow().category,
                    state_c.borrow().input_unit,
                    state_c.borrow().output_unit,
                );
                run_conversion(
                    &state_c,
                    &input_buffer_c,
                    &output_buffer_c,
                    &error_label_c,
                );
            }
        });

        input_unit_dropdown.connect_notify_local(Some("selected"), {
            let state_c = state_c.clone();
            let output_unit_dropdown_c = output_unit_dropdown_c.clone();
            let output_buffer_c = output_buffer_c.clone();
            let error_label_c = error_label_c.clone();
            let input_buffer_c = input_buffer_c.clone();
            move |dropdown, _| {
                let idx = dropdown.selected();
                if idx == gtk4::INVALID_LIST_POSITION {
                    return;
                }
                let units = Unit::units_for_category(state_c.borrow().category);
                let unit = units[idx as usize];
                let mut s = state_c.borrow_mut();
                s.input_unit = unit;
                s.output_unit = auto_output_unit(unit);
                drop(s);
                set_dropdown_selection(
                    &output_unit_dropdown_c,
                    state_c.borrow().category,
                    state_c.borrow().output_unit,
                );
                run_conversion(
                    &state_c,
                    &input_buffer_c,
                    &output_buffer_c,
                    &error_label_c,
                );
            }
        });

        output_unit_dropdown.connect_notify_local(Some("selected"), {
            let state_c = state_c.clone();
            let output_buffer_c = output_buffer_c.clone();
            let error_label_c = error_label_c.clone();
            let input_buffer_c = input_buffer_c.clone();
            move |dropdown, _| {
                let idx = dropdown.selected();
                if idx == gtk4::INVALID_LIST_POSITION {
                    return;
                }
                let units = Unit::units_for_category(state_c.borrow().category);
                let mut s = state_c.borrow_mut();
                s.output_unit = units[idx as usize];
                drop(s);
                run_conversion(
                    &state_c,
                    &input_buffer_c,
                    &output_buffer_c,
                    &error_label_c,
                );
            }
        });

        swap_btn.connect_clicked({
            let state_c = state_c.clone();
            let input_unit_dropdown_c = input_unit_dropdown_c.clone();
            let output_unit_dropdown_c = output_unit_dropdown_c.clone();
            let output_buffer_c = output_buffer_c.clone();
            let error_label_c = error_label_c.clone();
            let input_buffer_c = input_buffer_c.clone();
            move |_| {
                let mut s = state_c.borrow_mut();
                let in_u = s.output_unit;
                let out_u = s.input_unit;
                s.input_unit = in_u;
                s.output_unit = out_u;
                let cat = s.category;
                drop(s);
                refresh_unit_dropdowns(
                    &input_unit_dropdown_c,
                    &output_unit_dropdown_c,
                    cat,
                    in_u,
                    out_u,
                );
                run_conversion(
                    &state_c,
                    &input_buffer_c,
                    &output_buffer_c,
                    &error_label_c,
                );
            }
        });

        input_buffer.connect_changed({
            let state_c = state_c.clone();
            let input_buffer_c = input_buffer.clone();
            let output_buffer_c = output_buffer_c.clone();
            let error_label_c = error_label_c.clone();
            move |_| {
                run_conversion(&state_c, &input_buffer_c, &output_buffer_c, &error_label_c);
            }
        });

        precision_spin.connect_value_changed({
            let state_c = state_c.clone();
            let output_buffer_c = output_buffer_c.clone();
            let error_label_c = error_label_c.clone();
            let input_buffer_c = input_buffer_c.clone();
            move |spin| {
                let mut s = state_c.borrow_mut();
                s.prefs.decimal_places = spin.value() as u32;
                s.prefs.clamp_decimal_places();
                drop(s);
                save_prefs(&state_c);
                run_conversion(
                    &state_c,
                    &input_buffer_c,
                    &output_buffer_c,
                    &error_label_c,
                );
            }
        });
    }

    window.present();
}

fn category_drop_down() -> DropDown {
    let labels: Vec<&str> = UnitCategory::ALL.iter().map(|c| c.label()).collect();
    let strings: Vec<&str> = labels.iter().map(|s| *s).collect();
    DropDown::from_strings(&strings)
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

/// Pick a sensible output unit in the opposite measurement system when possible.
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
