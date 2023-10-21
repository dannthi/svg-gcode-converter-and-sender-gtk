use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::str;

use gtk4 as gtk;
use gtk::prelude::*;
use gtk::{gio, glib, Application, ApplicationWindow, Builder, Button, FileDialog, ResponseType, TextView, FileFilter};

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

    send_button.connect_clicked(glib::clone!(@weak window, @weak text_view => move |_|{
        // TODO
        return;
    }));

    open_button.connect_clicked(glib::clone!(@weak window, @weak text_view => move |_| {
        // Create a new file chooser dialog
        let filter = gtk::FileFilter::new();
        filter.add_mime_type("image/svg+xml"); //See: https://stackoverflow.com/questions/11918977/right-mime-type-for-svg-images-with-fonts-embedded
        filter.set_name(Some("SVG File"));
        
        let filters = gio::ListStore::new::<gtk::FileFilter>();
        filters.append(&filter);

        let file_chooser = gtk::FileDialog::builder()
            .title("Choose SVG-File")
            .accept_label("Open")
            .filters(&filters)
            .build();
    
        // Connect the response signal to handle the user's choice
        file_chooser.open(Some(&window), gio::Cancellable::NONE, move |file| {
            if let Ok(file) = file {
                let filename = file.path().expect("Couldn't get file path");
                let file = File::open(filename).expect("Couldn't open file");

                let mut reader = BufReader::new(file);
                let mut contents = String::new();
                let _ = reader.read_to_string(&mut contents);

                let g_code_vec = file_converter(contents);

                let mut g_code_string = String::new();

                for x in g_code_vec.iter() {
                    g_code_string.push_str(x.as_str());
                    g_code_string.push_str("\n");
                }

                text_view.buffer().set_text(&g_code_string);
            }
        });        
    }));


    window.present();

}


pub fn file_converter(contents: String) -> Vec<String> {
    let mut document = Document::parse("");
    let mut string_vector: Vec<String> = Vec::new();
    
    document = Document::parse(&contents.as_str());
    
    // Convert text of SVG-File to G-Code and display it in text_view
    let config = ConversionConfig::default();
    let options = ConversionOptions::default();
    let machine = Machine::default();

    let program =
        svg2program(&document.unwrap(), &config, options, machine);

    for x in program.iter() {
        string_vector.push(x.to_string());
    }

    return string_vector;
}

