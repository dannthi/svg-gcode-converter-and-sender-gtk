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
use std::rc::Rc;
use std::cell::RefCell;
use std::io::{Error, ErrorKind, Result};
use std::time::Duration;
use std::thread;

use grbli::device::response::firmware::driver;
use grbli::device::response::state::gcode_state;
// gtk imports
use gtk4 as gtk;
use gtk::prelude::*;
use gtk::{gio, glib, Application, ApplicationWindow, Builder, Button, DropDown, TextView, StringList, glib::GString, Window};
// additional imports
use svg2gcode::{svg2program, ConversionOptions, ConversionConfig, Machine};
use grbli::service::device_service::{DeviceService, DeviceEndpointType};
use grbli::device::command::{state, settings};
use grbli::device::DeviceInfo;

use roxmltree::Document;

fn main() { 
    // Default GTK setup
    let application = Application::new(
        Some("de.dhbw.lasergraviermaschine"),
        Default::default(),
    );
    application.connect_startup(setup_shortcuts);
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
    let port_dropdown_list: StringList = builder.object("list_ports").expect("Couldn't get builder");
    let port_dropdown: DropDown = builder.object("dropdown_ports").expect("Couldn't get builder");
    let help_button: Button = builder.object("info_button").expect("Couldn't get builder");
    let command_open_button: Button = builder.object("command_open_button").expect("Couldn't get builder");
    
    // Declare Vector where g-code will be stored in
    let g_code_vec: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

    // Scan for serial ports
    let ports = DeviceService::get_available_devices();
    println!("Available ports: {:?}", ports);

    for (port, _) in ports {
        port_dropdown_list.append(&port);
    }


    help_button.connect_clicked(|_| {
        // Show help window
        let ui_src = include_str!("help_viewer.ui");
        let id = "help_window"; 
        let (help_window, _) = build_generic(ui_src, id);
        help_window.present();
    });

    command_open_button.connect_clicked(glib::clone!(@weak port_dropdown_list, @weak port_dropdown  => move |_| {
        // When commandwindow button is clicked, open window
        let ui_src = include_str!("command_viewer.ui");
        let id = "command_window";
        let (command_window, command_builder) = build_generic(ui_src, id);
        let device_info_view: TextView = command_builder.object("device_info").expect("Couldn't get device_info_view");
        
        let val = set_communication(false, &port_dropdown_list, &port_dropdown);

        let mut service: DeviceService;
        let device_desc: (String, DeviceEndpointType);
        let id : &str;

        match val {
            Ok((desc, srv)) => {
                device_desc = desc;
                service = srv;
                id = &device_desc.0;
            },
            Err(e) => {
                eprintln!("Application error: {e}");
                service = DeviceService::new();
                id = "";
                return;
            }
        }

        let id = Rc::new(id.to_string());

        let mut service_move = Rc::new(RefCell::new(service));
        let mut service_status = service_move.clone();

        get_move_button(&command_builder, service_move, Rc::clone(&id));

        // read status from cnc
        let info = get_status(&command_builder, service_status, Rc::clone(&id));

        // println!("{:?}", info);

        let id = info.id();
        let firmware = {
            let buf = info.firmware_info();
            let buf = buf.version().unwrap();
            (buf.version(), buf.name())
        };
        let driver = info.machine_info();
        let gcode_state = info.gcode_state();

        // define String to store infos cnc is providing
        let text_buffer = format!("id: {}\n\
        firmware name: {}\n\
        firmware version: {}\n\
        machine state: {:?}\n\
        G-Code state: {:?}\n\
        ", id, firmware.1, firmware.0, driver, gcode_state);
        
        device_info_view.buffer().set_text(text_buffer.as_str());

        command_window.present();
    }));

    send_button.connect_clicked(glib::clone!(@weak g_code_vec, @weak port_dropdown_list, @weak port_dropdown => move |_|{
        let g_code_vec = g_code_vec.lock().expect("Mutex lock failed").clone();
        // let port_dropdown_list: StringList = builder.object("list_ports").expect("Couldn't get builder");
        // let port_dropdown: DropDown = builder.object("dropdown_ports").expect("Couldn't get builder");

        // Get selected port
        let selected_port_option = port_dropdown.selected();
        let port_option = port_dropdown_list.string(selected_port_option);
        if g_code_vec.is_empty() {
            // Refuse if no svg-file was imported
            println!("Please import SVG-File first");
            return;
        }
        else if port_option.is_none() {
            // Refuse if no port was selected
            println!("Please select a port with a connected GrblHAL CNC");
            return;
        }
        else {
            // Establish connection to selected port
            let val = set_communication(g_code_vec.is_empty(), &port_dropdown_list, &port_dropdown);

            println!("Send to port: {:?}\n", port_option);
            println!("Send G-Code: {:?}", g_code_vec);

            let mut service: DeviceService;
            let device_desc: (String, DeviceEndpointType);

            match val {
                Ok((desc, srv)) => {
                    device_desc = desc;
                    service = srv;
                    let id = &device_desc.0;
                    //Send G-Code here
                },
                Err(e) => {
                    println!("{:?}", e);
                    return;
                }
            }
        }
        return;
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
        
                // safe gcode as string
                for x in temp_g_code_vec.iter() {
                    g_code_string.push_str(x.as_str());
                    // write coordinates in same line as G0 or G1
                    if x.as_str() == "G0" || x.as_str() == "G1" {
                        // g_code_string.push_str(" ");
                    }
                    // check if first element of str is X
                    else if x.as_str().starts_with("X") {
                        // g_code_string.push_str(" ");
                    }
                    else if x.as_str() == "G90" {
                        g_code_string.push_str("\nG17\nG94\n");
                    }
                    else{
                        g_code_string.push_str("\n");
                    }
                }

                // write filename and gcodestring into textfields
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

fn setup_shortcuts(app: &Application) {
    // Set shortcuts -- doesn't work
    app.set_accels_for_action("window.destroy", &["<Ctrl>w"]);
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

fn build_generic(ui_src: &str, id: &str) -> (Window, Builder) {
    let generic_builder = Builder::new();
    generic_builder.add_from_string(ui_src).expect("Couldn't add from string");
    let generic_window: Window = generic_builder.object(id).expect("Couldn't get window");

    (generic_window, generic_builder)
}

fn set_communication(g_vec_bool: bool, port_dropdown_list: &StringList, port_dropdown: &DropDown) -> Result<((String, DeviceEndpointType), DeviceService)> {
    // Get selected port
    let selected_port_nr = port_dropdown.selected();
    let port_option = port_dropdown_list.string(selected_port_nr);

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
        let (device_desc, service) = setup_communication_handler(port_option);
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

    return filters;
}

fn manual_move(service: &mut DeviceService, id: &String, x: f32, y: f32) {
    if x == 1.0 {
        service.write_device_command(&id, format!("$J=G21G91X1F100\n").as_str()).unwrap();
    }
    else if x == -1.0 {
        service.write_device_command(&id, format!("$J=G21G91X-1F100\n").as_str()).unwrap();
    }
    else if y == 1.0 {
        service.write_device_command(&id, format!("$J=G21G91Y1F100\n").as_str()).unwrap();
    }
    else if y == -1.0 {
        service.write_device_command(&id, format!("$J=G21G91Y-1F100\n").as_str()).unwrap();
    }
    else {
        println!("Error");
    }
}

fn get_move_button(builder: &Builder, service: Rc<RefCell<DeviceService>>, id: Rc<String>) {
    let front_button: Button = builder.object("front").expect("Couldn't get builder");
    let left_button: Button = builder.object("left").expect("Couldn't get builder");
    let right_button: Button = builder.object("right").expect("Couldn't get builder");
    let back_button: Button = builder.object("back").expect("Couldn't get builder");

    front_button.connect_clicked(glib::clone!(@strong service, @strong id => move |_| {
        let mut service = service.borrow_mut();
        manual_move(&mut service, &id, 1.0, 0.0);
        println!("Move to front");
    }));

    left_button.connect_clicked(glib::clone!(@strong service, @strong id => move |_| {
        let mut service = service.borrow_mut();
        manual_move(&mut service, &id, 0.0, -1.0);
        println!("Move to left");
    }));

    right_button.connect_clicked(glib::clone!(@strong service, @strong id => move |_| {
        let mut service = service.borrow_mut();
        manual_move(&mut service, &id, 0.0, 1.0);
        println!("Move to right");
    }));

    back_button.connect_clicked(glib::clone!(@strong service, @strong id => move |_| {
        let mut service = service.borrow_mut();
        manual_move(&mut service, &id, -1.0, 0.0);
        println!("Move to back");
    }));
    
}

fn get_status(builder: &Builder, service: Rc<RefCell<DeviceService>>, id: Rc<String>) -> DeviceInfo {
    let mut service = service.borrow_mut();
    service.write_device_command(&id, format!("{}\n", state::GET_INFO_EXTENDED).as_str()).unwrap();
    service.write_device_command(&id, format!("{}\n", settings::GET_ALL).as_str()).unwrap();
    service.write_device_command(&id, format!("{}\n", settings::GET_DETAILS).as_str()).unwrap();
    service.write_device_command(&id, format!("{}\n", settings::GET_GROUPS).as_str()).unwrap();

    thread::sleep(Duration::from_millis(100)); // give uC time to process

    let info =  service.get_device_info(&id);
    println!("{:#?}", info);

    info.unwrap()
}
