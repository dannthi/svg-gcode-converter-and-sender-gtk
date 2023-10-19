use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::str;

use std::cell::RefCell;
use std::rc::Rc;

use gtk4 as gtk;
use gtk::glib;
use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, Builder, Button, FileChooserAction, FileChooserDialog, ResponseType, TextView, FileFilter};

use svg2gcode::{svg2program, ConversionOptions, ConversionConfig, Machine};
use roxmltree::Document;

fn main() { // Default GTK setup
    let application = Application::new(
        Some("com.example.myapp"),
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

        let document = Rc::new(RefCell::new(Document::parse("")));

        

        file_chooser.connect_response({
            let cloned_document = Rc::clone(&document);
                    
            move |d: &FileChooserDialog, response: ResponseType| {
                if response == ResponseType::Ok {
                    let file = d.file().expect("Couldn't get file");
        
                    let filename = file.path().expect("Couldn't get file path");
                    let file = File::open(&filename.as_path()).expect("Couldn't open file");
        
                    let mut reader = BufReader::new(file);
                    let mut contents = String::new();
                    let _ = reader.read_to_string(&mut contents);
        
                    println!("File contents:\n{}", contents);
        
                    let parsed_document = Document::parse(&contents.as_str());
                    if let Ok(parsed_doc) = parsed_document {
                        *cloned_document.borrow_mut() = parsed_document;
                    } else {
                        println!("Error parsing SVG file");
                        return;
                    }            
                }
        
                d.close();
            }
        });

        // Convert text of SVG-File to G-Code and display it in text_view
        let config = ConversionConfig::default();
        let options = ConversionOptions::default();
        let machine = Machine::default();

        let document_contents = document.borrow();

        let program =
            svg2program(&document_contents.unwrap(), &config, options, machine);

        let mut string = String::new();

        for x in program.iter() {
            // Append the current element to the existing text in the text_view
            string.push_str(x.to_string().as_str());
            // Append a newline to separate each element on a new line
            string.push_str("\n");
        }
        text_view.buffer().set_text(&string);

        file_chooser.show();
    }));

    window.show();
}



// Connect the response signal to handle the user's choice
        // file_chooser.connect_response(move |d: &FileChooserDialog, response: ResponseType| {
        //     if response == ResponseType::Ok {
        //         let file = d.file().expect("Couldn't get file");

        //         let filename = file.path().expect("Couldn't get file path");
        //         let file = File::open(&filename.as_path()).expect("Couldn't open file");

        //         let mut reader = BufReader::new(file);
        //         let mut contents = String::new();
        //         let _ = reader.read_to_string(&mut contents);

        //         println!("File contents:\n{}", contents);

        //         document = Document::parse(&contents.as_str());
        //         if document.is_err() {
        //             println!("Error parsing SVG file");
        //             return;
        //         }            
        //     }

        //     d.close();
        // });