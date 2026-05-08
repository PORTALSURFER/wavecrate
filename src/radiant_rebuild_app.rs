//! Radiant-first Sempal application rebuilt incrementally beside the legacy sample.

use radiant::prelude as ui;

/// Run the new Radiant-first application shell.
pub(crate) fn run() -> Result<(), String> {
    radiant::window("Sempal")
        .size(960, 540)
        .min_size(640, 360)
        .run(view())
}

fn view() -> ui::View<()> {
    ui::column([top_status_bar(), center_panel(), bottom_status_bar()])
        .spacing(0.0)
        .fill()
}

fn top_status_bar() -> ui::View<()> {
    ui::row([
        ui::text("Sempal").height(20.0).width(120.0),
        ui::text("Radiant rebuild").height(20.0).fill_width(),
        ui::text("ready").height(20.0).width(80.0),
    ])
    .spacing(8.0)
    .padding_x(12.0)
    .padding_y(4.0)
    .fill_width()
    .height(30.0)
}

fn center_panel() -> ui::View<()> {
    ui::row([folder_sidebar(), main_area()])
        .spacing(8.0)
        .padding(12.0)
        .fill()
}

fn folder_sidebar() -> ui::View<()> {
    ui::column([
        ui::text("Folders").height(22.0).fill_width(),
        ui::spacer().fill(),
    ])
    .spacing(6.0)
    .padding(8.0)
    .width(260.0)
    .fill_height()
}

fn main_area() -> ui::View<()> {
    ui::column([main_toolbar(), waveform_panel(), sample_browser()])
        .spacing(8.0)
        .padding(8.0)
        .fill()
}

fn main_toolbar() -> ui::View<()> {
    ui::row([
        ui::text("Source").height(22.0).width(80.0),
        ui::text("No folder loaded").height(22.0).fill_width(),
        ui::text("0 selected").height(22.0).width(96.0),
    ])
    .spacing(8.0)
    .padding_x(8.0)
    .padding_y(3.0)
    .style(ui::WidgetStyle::default())
    .fill_width()
    .height(34.0)
}

fn waveform_panel() -> ui::View<()> {
    ui::column([
        ui::text("Waveform").height(20.0).fill_width(),
        ui::spacer().fill(),
    ])
    .spacing(4.0)
    .padding(8.0)
    .style(ui::WidgetStyle::default())
    .fill_width()
    .height(150.0)
}

fn sample_browser() -> ui::View<()> {
    ui::column([
        sample_browser_header(),
        ui::spacer().fill(),
        sample_browser_status(),
    ])
    .spacing(0.0)
    .padding(8.0)
    .style(ui::WidgetStyle::default())
    .fill()
}

fn sample_browser_header() -> ui::View<()> {
    ui::row([
        ui::text("Name").height(22.0).fill_width(),
        ui::text("Type").height(22.0).width(120.0),
        ui::text("Length").height(22.0).width(90.0),
        ui::text("Tags").height(22.0).width(140.0),
    ])
    .spacing(8.0)
    .padding_x(6.0)
    .fill_width()
    .height(28.0)
}

fn sample_browser_status() -> ui::View<()> {
    ui::row([
        ui::text("Browser").height(20.0).width(90.0),
        ui::text("Samples will be listed here").height(20.0).fill_width(),
    ])
    .spacing(8.0)
    .padding_x(6.0)
    .fill_width()
    .height(28.0)
}

fn bottom_status_bar() -> ui::View<()> {
    ui::row([
        ui::text("0 samples").height(20.0).width(120.0),
        ui::text("No source loaded").height(20.0).fill_width(),
    ])
    .spacing(8.0)
    .padding_x(12.0)
    .padding_y(4.0)
    .fill_width()
    .height(30.0)
}
