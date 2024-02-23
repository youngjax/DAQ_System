#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use rusqlite::{Connection, Result};
use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints};
use chrono::{NaiveDateTime, Utc};

 
//WINDOW SETUP VARIABLES
static WINDOW_WIDTH: f32 = 800.0;
static WINDOW_HEIGHT: f32 = 600.0;
static ROUNDING: f32 = 5.0;
static BORDER_WIDTH: f32 = 1.0;
static INNER_MARGIN: f32 = 10.0;
static OUTER_MARGIN: f32 = 10.0;
static DATA_WIDTH: f32 = 580.0; //width of right-half GUI (data visualization section)

static CONFIG_WIDTH: f32 = 180.0; //MAKE SURE TO UPDATE THIS IF YOU CHANGE WINDOW_WIDTH, DATA_WIDTH, OR OUTER_MARGIN! (use WINDOW_WIDTH - DATA_WIDTH - (4.0 * OUTER_MARGIN))

enum SensorType {
    ADC,
    GPS,
    MKR,
}

//returns a vector with all the values from the specifed table
fn fetch_all_data(conn: &Connection, sensor_type: SensorType, display_frame: f64) -> Result<Vec<(NaiveDateTime, f64, f64, f64)>> {
    let pull_table = match sensor_type {
        SensorType::ADC => "adc_data",
        SensorType::GPS => "gps_data",
        SensorType::MKR => "mkr_data",
    };

    let query = format!("SELECT * FROM {} WHERE recording_time >= datetime('now', '-{} minutes')", pull_table, display_frame);
    let mut stmt = conn.prepare(&query)?;

    let rows = stmt.query_map([], |row| {
        let id: f64 = row.get(0)?;
        let recording_time: String = row.get(1)?;
        let data_1: f64 = row.get(2)?;
        let data_2: Option<f64> = row.get(3)?;

        let recording_time_struct = NaiveDateTime::parse_from_str(recording_time.as_str(), "%Y-%m-%d %H:%M:%S").unwrap();

        let data_2_float = data_2.unwrap_or(0.0);

        Ok((recording_time_struct, id, data_1, data_2_float))
    })?;
    let mut data = Vec::new();
    for row in rows {
        data.push(row?);
    }
    Ok(data)
}

fn display_data_table(ui: &mut egui::Ui, conn: &Connection, sensor_type: SensorType, scroll_area_id: &str, display_frame: f64) {
    let scroll_area_id = ui.make_persistent_id(scroll_area_id); // Generate a unique identifier for the ScrollArea

    egui::ScrollArea::vertical().id_source(scroll_area_id).show(ui, |ui| {
        match fetch_all_data(&conn, sensor_type, display_frame) {
            Ok(data) => {
                for row in data {
                    ui.label(format!("Recording Time: {}, ID: {}, Data_1: {}, Data_2: {}", row.0, row.1, row.2, row.3));
                }
            }
            Err(err) => {
                eprintln!("display_data_table FAILED: {}", err);
                std::process::exit(1);
            }
        }
    });
}

/*make a plot, last argument chooses if you're plotting:
axis:    y        x
    0. data_1 v time
    1. data_2 v time
    2. data_1 v data_2
    3. data_2 v data_1
*/
fn display_data_line_plot(ui: &mut egui::Ui, conn: &Connection, sensor_type: SensorType, scroll_area_id: &str, plot_type: usize, display_frame: f64) {
    let scroll_area_id = ui.make_persistent_id(scroll_area_id); // Generate a unique identifier for the ScrollArea
    egui::ScrollArea::vertical().id_source(scroll_area_id).show(ui, |ui| {
        match fetch_all_data(&conn, sensor_type, display_frame) {
            Ok(data) => {
                let plot_data: PlotPoints = match plot_type {

                    0 => data.into_iter().map(|(recording_time, _, y, _)| {
                        let float_time = -Utc::now().naive_utc().signed_duration_since(recording_time).num_seconds() as f64;
                        [float_time, y]
                    }).collect(),
                    1 => data.into_iter().map(|(recording_time, _, _, y)| {
                        let float_time = -Utc::now().naive_utc().signed_duration_since(recording_time).num_seconds() as f64;
                        [float_time, y]
                    }).collect(),
                    2 => data.into_iter().map(|(_, _, y, x) | [x, y]).collect(),
                    3 => data.into_iter().map(|(_, _, x, y) | [x, y]).collect(),
                    _ => {
                        eprintln!("EASY ERROR: Invalid plot type: {}", plot_type);
                        std::process::exit(1);
                    }
                };
                let line_plot = Line::new(plot_data);
                Plot::new(scroll_area_id).view_aspect(2.0).show(ui, |plot_ui| plot_ui.line(line_plot)); // Display line plot
            }
            Err(err) => {
                eprintln!("display_data_line_plot FAILED: {}", err);
                std::process::exit(1);
            }
        }
    });
}

fn clear_table_data(conn: &Connection, sensor_type: SensorType) -> Result<(), rusqlite::Error> {
    let pull_table = match sensor_type {
        SensorType::ADC => "adc_data",
        SensorType::GPS => "gps_data",
        SensorType::MKR => "mkr_data",
    };
    let query = format!("DELETE FROM {}", pull_table);
    conn.execute(&query, [])
        .map(|_| ())
}


fn main() -> Result<(), eframe::Error> {
    //Set UI size:
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([WINDOW_WIDTH, WINDOW_HEIGHT]),
        ..Default::default()
    };

    //start main loop
    eframe::run_native(
        "DAQ GUI Version 1.0",
        options,
        Box::new(|_cc| {
            // No need for image support, so we don't do anything here
            Box::<MyApp>::default()
        }),
    )
}

struct MyApp {
    adc_selected: bool,
    gps_selected: bool,
    mkr_selected: bool,
    selected_adc_output_style: OutputStyle, //default adc output style
    selected_gps_output_style: OutputStyle, //default gps output style
    selected_mkr_output_style: OutputStyle, //default mkr output style
    time_to_collect: i32,
    display_frame: f64,
}

#[derive(PartialEq)]
enum OutputStyle {
    Table,
    Graph,
}


impl Default for MyApp { //used to initialize MyApp struct instances
    fn default() -> Self { //returns an instance of MyApp with the given values
        Self {
            adc_selected: false,
            gps_selected: false,
            mkr_selected: false,
            selected_adc_output_style: OutputStyle::Table, //default adc output style
            selected_gps_output_style: OutputStyle::Graph, //default gps output style
            selected_mkr_output_style: OutputStyle::Table, //default mkr output style
            time_to_collect: 30,
            display_frame: 5.0,
        }
    }
}
impl eframe::App for MyApp {
    // &mut self                  -- mutable reference to the MyApp instance, allowing you to modify its state
    // ctx: &egui::Context        -- reference to the egui context, which provides access to UI functionalities
    // _frame: &mut eframe::Frame -- mutable reference to the eframe frame
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {

        let conn = match rusqlite::Connection::open("database_2/the_database.db") {
            Ok(conn) => conn,
            Err(err) => {
                eprintln!("\n\n\nDATABASE DIDNT WORK: {}\n\n\n", err);
                std::process::exit(1);
            }
        };

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("DAQ GUI");

            ui.horizontal(|ui| { // this will allow for left and right halves next to each other
                ui.allocate_ui( // set height and width of left half
                    egui::vec2(
                        CONFIG_WIDTH,
                        WINDOW_HEIGHT,
                    ),
                    |ui| { 
                        ui.vertical(|ui| { // now this is the left side vertical container, we'll have sensor selection on top, placeholder on bottom

                            /* SETUP SECTION */

                            egui::Frame::none() // put a frame around the section
                                // .fill(egui::Color32::RED) // fills block with color (maybe do it in the future?)
                                .fill(egui::Color32::LIGHT_GRAY)
                                .stroke(egui::Stroke::new(BORDER_WIDTH, egui::Color32::BLACK)) // makes 1px border
                                .rounding(ROUNDING) // sets ROUNDING to 5 pixels
                                .inner_margin(INNER_MARGIN) // give some space around contents
                                .outer_margin(OUTER_MARGIN) // give some space around contents
                                .show(ui, |ui| {

                                    ui.heading("Select Sensors:");

/*ISSUE: Sizing defaults to min instead of filling the whole space.
    one fix: add a text_edit
*/
                                    ui.add_space(15.0); // pixels between content
                                    ui.checkbox(&mut self.adc_selected, "ADC sensor");   // Checkbox for ADC sensor
                                    if self.adc_selected {
                                        ui.vertical(|ui| {
                                            ui.selectable_value(&mut self.selected_adc_output_style, OutputStyle::Table, "\tTable");
                                            ui.selectable_value(&mut self.selected_adc_output_style, OutputStyle::Graph, "\tGraph");
                                        });
                                    }

                                    ui.checkbox(&mut self.gps_selected, "GPS sensor");   // Checkbox for GPS sensor
                                    if self.gps_selected {
                                        ui.vertical(|ui| {
                                            ui.selectable_value(&mut self.selected_gps_output_style, OutputStyle::Table, "\tTable");
                                            ui.selectable_value(&mut self.selected_gps_output_style, OutputStyle::Graph, "\tGraph");
                                        });
                                    }

                                    ui.checkbox(&mut self.mkr_selected, "MIKROE sensor"); // Checkbox for MIKROE sensor
                                    if self.mkr_selected {
                                        ui.vertical(|ui| {
                                            ui.selectable_value(&mut self.selected_mkr_output_style, OutputStyle::Table, "\tTable");
                                            ui.selectable_value(&mut self.selected_mkr_output_style, OutputStyle::Graph, "\tGraph");
                                        });
                                    }
                                });


                            /* CONFIG/RECORDING SECTION */

                            egui::Frame::none() // put a frame around the section
                                .fill(egui::Color32::LIGHT_BLUE) // fills block with color
                                .stroke(egui::Stroke::new(BORDER_WIDTH, egui::Color32::BLACK)) // makes 1px border
                                .rounding(ROUNDING) // sets ROUNDING to 5 pixels
                                .inner_margin(INNER_MARGIN) // give some space around contents
                                .outer_margin(OUTER_MARGIN) // give some space around contents
                                .show(ui, |ui| {
                                    ui.heading("Configuration");
                                    ui.add_space(15.0);

                                    ui.label("Save data for...");
                                    ui.style_mut().spacing.slider_width = 75.0;
                                    ui.add(egui::Slider::new(&mut self.time_to_collect, 1..=120).text("Min")); // In the future, report self.time_to_collect to the logger code

                                    ui.add_space(15.0); // pixels between content
                                    ui.horizontal(|ui| {
                                        ui.label("data display frame:");
                                        ui.add(egui::DragValue::new(&mut self.display_frame)
                                            .clamp_range(0.0..=f64::INFINITY) // Set the minimum value to 0
                                            .speed(0.1) // Set how fast value changes
                                        );
                                    });

                                    ui.add_space(15.0); // pixels between content

                                    if self.adc_selected {
                                        if ui.add(egui::Button::new("Clear adc data")).clicked() {
                                            if let Err(err) = clear_table_data(&conn, SensorType::ADC) {
                                                eprintln!("Failed to clear ADC table data: {}", err);
                                    }}}
                                    if self.gps_selected {
                                        if ui.add(egui::Button::new("Clear gps data")).clicked() {
                                            if let Err(err) = clear_table_data(&conn, SensorType::GPS) {
                                                eprintln!("Failed to clear GPS table data: {}", err);
                                    }}}
                                            if self.mkr_selected {
                                        if ui.add(egui::Button::new("Clear mkr data")).clicked() {
                                            if let Err(err) = clear_table_data(&conn, SensorType::MKR) {
                                                eprintln!("Failed to clear MKR table data: {}", err);
                                    }}}
                                });
                        });
                    }
                );
                // Right half
                ui.vertical(|ui| {
                    // Determine how many sensors are selected
                    let num_selected = if self.adc_selected { 1 } else { 0 }
                                     + if self.gps_selected { 1 } else { 0 }
                                     + if self.mkr_selected { 1 } else { 0 };

                    // Check if any sensor is selected
                    if num_selected > 0 {
                        let block_height = (WINDOW_HEIGHT / num_selected as f32) - (2.0 * OUTER_MARGIN); // Calculate the height of each sensor block

                        if self.adc_selected { // Render ADC sensor block if selected
                            ui.allocate_ui(
                                egui::vec2(DATA_WIDTH, block_height),
                                |ui| {
                                    egui::Frame::none()
                                        .fill(egui::Color32::LIGHT_GREEN)
                                        .stroke(egui::Stroke::new(BORDER_WIDTH, egui::Color32::BLACK))
                                        .rounding(ROUNDING)
                                        .inner_margin(INNER_MARGIN) // give some space around contents
                                        .outer_margin(OUTER_MARGIN) // give some space around contents
                                        .show(ui, |ui| {
                                            ui.label("ADC Sensor Data");
                                            match self.selected_adc_output_style {
                                                OutputStyle::Table => display_data_table(ui, &conn, SensorType::ADC, "adc_scroll_area", self.display_frame),
                                                OutputStyle::Graph => display_data_line_plot(ui, &conn, SensorType::ADC, "adc_scroll_area", 0, self.display_frame),
                                            }
                                        });
                                },
                            );
                        }
                        
                        if self.gps_selected { // Render GPS sensor block if selected
                            ui.allocate_ui(
                                egui::vec2(DATA_WIDTH, block_height),
                                |ui| {
                                    egui::Frame::none()
                                    .fill(egui::Color32::LIGHT_RED)
                                    .stroke(egui::Stroke::new(BORDER_WIDTH, egui::Color32::BLACK))
                                    .rounding(ROUNDING)
                                    .inner_margin(INNER_MARGIN)
                                    .outer_margin(OUTER_MARGIN)
                                    .show(ui, |ui| {
                                        ui.label("GPS Sensor Data");
                                        match self.selected_gps_output_style {
                                            OutputStyle::Table => display_data_table(ui, &conn, SensorType::GPS, "gps_scroll_area", self.display_frame),
                                            OutputStyle::Graph => display_data_line_plot(ui, &conn, SensorType::GPS, "gps_scroll_area", 3, self.display_frame),
                                        }
                                    });
                                },
                            );
                        }

                        if self.mkr_selected { // Render MIKROE sensor block if selected
                            ui.allocate_ui(
                                egui::vec2(DATA_WIDTH, block_height), // try ui.available_size_before_wrap_finite().x instead of 600?
                                |ui| {
                                    egui::Frame::none()
                                        .fill(egui::Color32::LIGHT_YELLOW)
                                        .stroke(egui::Stroke::new(BORDER_WIDTH, egui::Color32::BLACK))
                                        .rounding(ROUNDING)
                                        .inner_margin(INNER_MARGIN) 
                                        .outer_margin(OUTER_MARGIN) 
                                        .show(ui, |ui| {
                                            ui.label("MIKROE Sensor Data");
                                            match self.selected_mkr_output_style {
                                                OutputStyle::Table => display_data_table(ui, &conn, SensorType::MKR, "mkr_scroll_area", self.display_frame),
                                                OutputStyle::Graph => display_data_line_plot(ui, &conn, SensorType::MKR, "mkr_scroll_area", 0, self.display_frame),
                                            }
                                        });
                                },
                            );
                        }
                    }
                });
            });
        });
    }
}

//examples:
//let string_label = ui.label("String: ");
//ui.text_edit_singleline(&mut self.filler_text).labelled_by(string_label.id);

//ui.add(egui::Slider::new(&mut self.filler_slider, 0..=100).text("slider!")); // slider that depends on the value of struct variable 'filler_slider'

//ui.label(format!("Sensor selection:\n\tADC {}\n\tGPS {}\n\tMKR {}", self.adc_selected, self.gps_selected, self.mkr_selected));