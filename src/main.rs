///////////////////////////////////////////////////
/// GUI for converting SVG-Files to G-Code 
/// and send them to GrblHAL CNCs
///////////////////////////////////////////////////

// std imports
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::str;
use std::sync::{Arc, Mutex};
use std::io::{Error, ErrorKind, Result};

// gtk imports
use gtk4 as gtk;
use gtk::prelude::*;
use gtk::{gio, glib, Application, ApplicationWindow, Builder, Button, DropDown, TextView, StringList, glib::GString, Window};

use serialport::SerialPortInfo;
// additional imports
use svg2gcode::{svg2program, ConversionOptions, ConversionConfig, Machine};
use grbli::service::device_service::{DeviceService, DeviceEndpointType};
use grbli::device::command::state::GET_INFO_EXTENDED;
use roxmltree::Document;
use serialport;

fn main() { 
    // Default GTK setup
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

    // Get objects from builder file
    let window: ApplicationWindow = builder.object("window").expect("Couldn't get window");
    window.set_application(Some(application));
    let open_button: Button = builder.object("open_button").expect("Couldn't get builder");
    let text_view: TextView = builder.object("text_view").expect("Couldn't get text_view");
    let filename_view: TextView = builder.object("filename_view").expect("Couldn't get filename_view");
    let send_button: Button = builder.object("send_button").expect("Couldn't get builder");
    // let port_dropdown_list: StringList = builder.object("list_ports").expect("Couldn't get builder");
    // let port_dropdown: DropDown = builder.object("dropdown_ports").expect("Couldn't get builder");
    let help_button: Button = builder.object("info_button").expect("Couldn't get builder");

    // Set shortcuts -- doesn't work somehow?
    // let close_window_trigger = ShortcutTrigger::parse_string("<Control>W").unwrap();
    // let close_window_action = ShortcutAction::parse_string("window.destroy").unwrap(); // hier fehler

    // let close_window_shortcut = Shortcut::new(Some(close_window_trigger), Some(close_window_action));
    // window.set_property("close shortcut", &close_window_shortcut);
    
    // Declare Vector where g-code will be stored in
    let g_code_vec: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

    // Scan for serial ports
        // Option 1
    let ports = DeviceService::get_available_devices();
    println!("Available ports: {:?}", ports);


        //Option 2
    let serial_ports: Vec<SerialPortInfo> = Vec::new();
    match serialport::available_ports() {
        Ok(serial_ports) => serial_ports.into_iter().collect(),//.filter(|port| matches!(port.port_type, serialport::SerialPortType::UsbPort(_))).collect(),
        Err(_) => Vec::new(),
    };
    println!("Available ports: {:?}", serial_ports);

    // for (port, _) in ports {
    //     println!("{:?}", port);
    //     port_dropdown_list.append(&port);
    // }

    help_button.connect_clicked(|_| {
        // When help button is clicked, open help window
        build_help_ui();
    });

    send_button.connect_clicked(glib::clone!(@weak g_code_vec => move |_|{
        let g_code_vec = g_code_vec.lock().expect("Mutex lock failed").clone();
        let port_dropdown_list: StringList = builder.object("list_ports").expect("Couldn't get builder");
        let port_dropdown: DropDown = builder.object("dropdown_ports").expect("Couldn't get builder");

        let val = set_communication(g_code_vec.is_empty(), port_dropdown_list, port_dropdown);

        let service: DeviceService;
        let device_desc: (String, DeviceEndpointType);

        match val {
            Ok((desc, srv)) => {
                device_desc = desc;
                service = srv;
        
                let info =  service.get_device_info(&device_desc.0).unwrap();
                println!("{:#?}", info);
            },
            Err(e) => {
                println!("{:?}", e);
            }
        }

        // // Get selected port
        // let selected_port_option = port_dropdown.selected();
        // let port_option = port_dropdown_list.string(selected_port_option);
        // if g_code_vec.is_empty() {
        //     // Refuse if no svg-file was imported
        //     println!("Please import SVG-File first");
        //     return;
        // }
        // else if port_option.is_none() {
        //     // Refuse if no port was selected
        //     println!("Please select a port with a connected GrblHAL CNC");
        //     return;
        // }
        // else{
        //     // Establish connection to selected port
        //     // let service = service.lock().expect("Mutex lock failed"); // get service from outside the scope
        //     // let mut service = DeviceService::new();
        //     // let mut device_desc: (String, DeviceEndpointType) = (String::new(), DeviceEndpointType::Serial);

        //     let (device_desc, mut service) = setup_communication_handler(port_option);

        //     service.write_device_command(&device_desc.0, format!("{}\n", GET_INFO_EXTENDED).as_str()).unwrap();

        //     let info =  service.get_device_info(&device_desc.0).unwrap();
        //     println!("{:#?}", info);
        // }

        // // println!{"{:?}", g_code_vec};
        // return;
    }));
    
    open_button.connect_clicked(glib::clone!(@weak window, @weak text_view, @weak filename_view => move |_| {
        // Clone g_code_vec to use and modify it in the closure
        let g_code_vec_clone = Arc::clone(&g_code_vec);

        let filters = init_filters();

        // Create a new file chooser dialog
        let file_chooser = gtk::FileDialog::builder()
            .title("Choose SVG-File")
            .accept_label("Open")
            .filters(&filters)
            .build();
        
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
    // Convert SVG-File to G-Code and returns vector with G-Code-Strings
    let mut string_vector: Vec<String> = Vec::new();
    let document = Document::parse(&contents.as_str());
    
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

pub fn setup_communication_handler(port_option: Option<GString>) -> ((String, DeviceEndpointType), DeviceService) {
    // Get port name
    let selected_port = port_option.unwrap();

    let mut service = DeviceService::new();

    // open the device connection on port
    let device_desc = (selected_port.to_string(), DeviceEndpointType::Serial);
    service.open_device(&device_desc).unwrap();

    (device_desc, service)
}

fn build_help_ui() {
    let help_ui_src = include_str!("help_viewer.ui");
    let help_builder = Builder::new();
    help_builder.add_from_string(help_ui_src).expect("Couldn't add from string");
    let help_window: Window = help_builder.object("help_window").expect("Couldn't get window");

    help_window.present();
}

fn set_communication(g_vec_bool: bool, port_dropdown_list: StringList, port_dropdown: DropDown) -> Result<((String, DeviceEndpointType), DeviceService)> {
    // Get selected port
    let selected_port_option = port_dropdown.selected();
    let port_option = port_dropdown_list.string(selected_port_option);

    if g_vec_bool {
        // Refuse if no svg-file was imported
        return Err(Error::new(ErrorKind::Other, "Please import SVG-File first"));

    }
    else if port_option.is_none() {
        // Refuse if no port was selected
        return Err(Error::new(ErrorKind::Other, "Please select a port with a connected GrblHAL CNC"));
    }
    else{
        // Establish connection to selected port
        // let service = service.lock().expect("Mutex lock failed"); // get service from outside the scope
        // let mut service = DeviceService::new();
        // let mut device_desc: (String, DeviceEndpointType) = (String::new(), DeviceEndpointType::Serial);

        let (device_desc, mut service) = setup_communication_handler(port_option);

        service.write_device_command(&device_desc.0, format!("{}\n", GET_INFO_EXTENDED).as_str()).unwrap();

        return Ok((device_desc, service))
    }    
}

fn init_filters() -> gio::ListStore {
    // Create filter to only show SVG-Files
    let filter = gtk::FileFilter::new();
    filter.add_mime_type("image/svg+xml"); //See: https://stackoverflow.com/questions/11918977/right-mime-type-for-svg-images-with-fonts-embedded
    filter.set_name(Some("SVG File"));
    
    let filters = gio::ListStore::new::<gtk::FileFilter>();
    filters.append(&filter);

    filters
}
