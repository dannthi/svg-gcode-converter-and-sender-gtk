use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::str;

use gtk4 as gtk;
use gtk::prelude::*;
use gtk::{glib, Application, ApplicationWindow, Builder, Button, FileChooserAction, FileChooserDialog, ResponseType, TextView, FileFilter};

use svg2gcode::{svg2program, ConversionOptions, ConversionConfig, Machine};
use roxmltree::Document;


fn main() { // Default GTK setup
    let application = Application::new(
        Some("de.dhbw.lasergraviermaschine"),
        Default::default(),
    );
    application.connect_activate(build_ui);
    application.run();
}

pub fn build_ui(application: &Application) {
    // Use xml as builder
    let ui_src = include_str!("text_viewer.ui");
    let builder = Builder::new();
    builder
        .add_from_string(ui_src)
        .expect("Couldn't add from string");

    let window: ApplicationWindow = builder.object("window").expect("Couldn't get window");
    window.set_application(Some(application));
    let open_button: Button = builder.object("open_button").expect("Couldn't get builder");
    let text_view: TextView = builder.object("text_view").expect("Couldn't get text_view");
    let send_button: Button = builder.object("send_button").expect("Couldn't get builder");

    // send_button.connect_clicked(glib::clone!(@weak window, @weak text_view => move |_|{
    //     // TODO
    //     return;
    // }));

    open_button.connect_clicked(glib::clone!(@weak window, @weak text_view => move |_| {
        // Create a new file chooser dialog
        let file_chooser = FileChooserDialog::new(
            Some("Choose SVG-File"),
            Some(&window),
            FileChooserAction::Open,
            &[("Open", ResponseType::Ok), ("Cancel", ResponseType::Cancel)],
        );

        // Add a filter to only show SVG files
        let filter = FileFilter::new();
        filter.add_mime_type("image/svg+xml"); //See: https://stackoverflow.com/questions/11918977/right-mime-type-for-svg-images-with-fonts-embedded
        filter.set_name(Some("SVG-Dateien"));
        file_chooser.add_filter(&filter);

        let mut string_vector: Vec<String> = Vec::new(); 
    
        // Connect the response signal to handle the user's choice
        file_chooser.connect_response(|file_chooser, ResponseType::Ok| {
            file_chooser_converter(file_chooser, &text_view, &mut string_vector)
        });       
        window.show();

    }));
}


pub fn file_chooser_converter(d: &FileChooserDialog, text_view: &TextView, string_vector: &mut Vec<String>) {
    let mut string = String::new();
    let mut document = Document::parse("");

    let file = d.file().expect("Couldn't get file");
    let filename = file.path().expect("Couldn't get file path");
    let file = File::open(&filename.as_path()).expect("Couldn't open file");

    let mut reader = BufReader::new(file);
    let mut contents = String::new();
    let _ = reader.read_to_string(&mut contents);

    println!("File contents:\n{}", contents);
    
    document = Document::parse(&contents.as_str());
    
    // Convert text of SVG-File to G-Code and display it in text_view
    let config = ConversionConfig::default();
    let options = ConversionOptions::default();
    let machine = Machine::default();

    let program =
        svg2program(&document.unwrap(), &config, options, machine);

    for x in program.iter() {
        // Append the current element to the existing text in the text_view
        string.push_str(x.to_string().as_str());
        // Append a newline to separate each element on a new line
        string.push_str("\n");
        string_vector.push(x.to_string());
    }

    text_view.buffer().set_text(&string);

    d.show();
    d.close();

    // return (d.clone(), ResponseType::Ok);
}

