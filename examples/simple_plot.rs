use egui::Context;
use egui_enum_plot::{EnumPlot, EnumPlotLine};
use egui_plot::{Line, Plot, PlotPoint, PlotPoints};
use std::fmt::Display;

enum IsSineGreaterThanZero {
    Yes,
    No,
}

impl Display for IsSineGreaterThanZero {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IsSineGreaterThanZero::Yes => write!(f, "Yes"),
            IsSineGreaterThanZero::No => write!(f, "No"),
        }
    }
}

impl PartialEq for IsSineGreaterThanZero {
    fn eq(&self, other: &Self) -> bool {
        core::mem::discriminant(self) == core::mem::discriminant(other)
    }
}

fn main() {
    let native_options = eframe::NativeOptions::default();
    let _ = eframe::run_native(
        "Example Enum Plot",
        native_options,
        Box::new(|cc| {
            Box::new(MyApp {
                ctx: cc.egui_ctx.clone(),
            })
        }),
    );
}

struct MyApp {
    ctx: Context,
}

impl eframe::App for MyApp {
    fn update(&mut self, _ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut analog_data = Vec::new();
        for x in 0..1000 {
            analog_data.push(PlotPoint::new(x as f64, f64::sin(x as f64 / 100.0)));
        }

        let mut bool_data = Vec::new();
        for value in &analog_data {
            bool_data.push((value.x, value.y > 0.0));
        }

        let mut enum_data = Vec::new();
        for value in &analog_data {
            let enum_value = if value.y > 0.0 {
                IsSineGreaterThanZero::Yes
            } else {
                IsSineGreaterThanZero::No
            };
            enum_data.push((value.x, enum_value))
        }

        egui::CentralPanel::default().show(&self.ctx, |ui| {
            let plot = Plot::new("Main Plot");
            let mut enum_plot = EnumPlot::new(ui);
            enum_plot.setup_enum_plot(|enum_plot_ui| {
                enum_plot_ui.add_line("Boolean".to_string(), EnumPlotLine::new(bool_data));
                enum_plot_ui.add_line("Enum".to_string(), EnumPlotLine::new(enum_data));
            });

            enum_plot.show(ui, plot, |plot_ui| {
                let plot_points = PlotPoints::Owned(analog_data);
                let line = Line::new(plot_points).name("Data");
                plot_ui.line(line);
            });
        });
    }
}
