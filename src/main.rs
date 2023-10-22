use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::str;

use gtk4 as gtk;
use gtk::prelude::*;
use gtk::{gio, glib, Application, ApplicationWindow, Builder, Button, TextView, DropDown};

use svg2gcode::{svg2program, ConversionOptions, ConversionConfig, Machine};
use serialport::{SerialPort, DataBits, StopBits, FlowControl, Parity};
use roxmltree::Document;
use std::sync::{Arc, Mutex};

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
    let filename_view: TextView = builder.object("filename_view").expect("Couldn't get filename_view");
    let send_button: Button = builder.object("send_button").expect("Couldn't get builder");
    let port_dropdown: DropDown = builder.object("list_ports").expect("Couldn't get builder");

    let g_code_vec: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

    let ports = serialport::available_ports().expect("No ports found!");
    println!("Available ports: {:?}", ports);
    port_dropdown.factory();

    send_button.connect_clicked(glib::clone!(@weak window, @weak text_view, @weak g_code_vec => move |_|{
        let g_code_vec = g_code_vec.lock().expect("Mutex lock failed").clone();

        if g_code_vec.is_empty() {
            println!("Please import SVG-File first");
            return;
        }
        else{
            let mut port = serialport::new("/dev/ttyACM0", 9600)
            .data_bits(DataBits::Eight)
            .flow_control(FlowControl::None)
            .parity(Parity::None)
            .stop_bits(StopBits::One)
            .timeout(std::time::Duration::from_millis(10))
            .open()
            .expect("Failed to open port");

            for x in g_code_vec.iter() {
                port.write(x.as_bytes()).expect("Write failed");
            }
        }

        println!{"{:?}", g_code_vec};
        return;
    }));

    open_button.connect_clicked(glib::clone!(@weak window, @weak text_view, @weak filename_view => move |_| {
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

        // Clone g_code_vec to use and modify it in the closure
        let g_code_vec_clone = Arc::clone(&g_code_vec);
        
        // Connect the response signal to handle the user's choice
        file_chooser.open(Some(&window), gio::Cancellable::NONE, move |file| {
            if let Ok(file) = file {
                // Get the g_code_vec from the closure
                let mut temp_g_code_vec = g_code_vec_clone.lock().unwrap();

                let filename = file.path().expect("Couldn't get file path");   
                let file = File::open(&filename).expect("Couldn't open file");
                let mut reader = BufReader::new(file);
                let mut contents = String::new();
                let _ = reader.read_to_string(&mut contents);

                // Call function to convert SVG-File to G-Code in string format
                *temp_g_code_vec = file_converter(contents);
                let mut g_code_string = String::new();
        
        
                for x in temp_g_code_vec.iter() {
                    g_code_string.push_str(x.as_str());
                    g_code_string.push_str("\n");
                }
                println!("{:?}", filename.to_str().unwrap());
                filename_view.buffer().set_text(&filename.to_str().unwrap());
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
