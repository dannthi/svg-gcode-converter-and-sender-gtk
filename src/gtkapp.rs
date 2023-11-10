use gtk::prelude::*;
use gtk::{gio, glib, Application, ApplicationWindow, Builder, Button, DropDown, TextView, StringList, glib::GString, Window};
use gtk4 as gtk;

pub struct GtkApp {
    pub window: ApplicationWindow,
    pub builder: Builder,
}

impl GtkApp {
    pub fn new(app: &Application) -> GtkApp {
        let window = ApplicationWindow::new(app);
        let builder = Builder::new();

        GtkApp { window, builder }
    }

    pub fn build_gtkwindow(&self, str: &str, app: &Application) {
        // Use xml as builder
        self.builder
            .add_from_string(&str)
            .expect("Couldn't add from string");

        self.window.set_application(Some(app));
    }

    pub fn build_add_window(&self, str: &str, app: &Application) {
        // Use xml as builder
        self.builder
            .add_from_string(&str)
            .expect("Couldn't add from string");

        self.window.set_application(Some(app));
    }


    pub fn init_filters(self, str_vec: Vec<String>) -> gio::ListStore {
        // Create filter to only show SVG-Files
        let filters = gio::ListStore::new::<gtk::FileFilter>();

        for str in str_vec {
            let filter = gtk::FileFilter::new();
            filter.add_mime_type(&str);
            // filter.set_name(Some(&str));
            filters.append(&filter);

        }
        // let filter = gtk::FileFilter::new();
        // filter.add_mime_type("image/svg+xml"); //See: https://stackoverflow.com/questions/11918977/right-mime-type-for-svg-images-with-fonts-embedded
        // filter.set_name(Some("SVG File"));
        
        filters
    }

    pub fn get_button(&self, button_name: &str) -> Button {
        self.builder
            .object(button_name)
            .expect("Couldn't get button")
    }

    pub fn get_textview(&self, textview_name: &str) -> TextView {
        self.builder
            .object(textview_name)
            .expect("Couldn't get textview")
    }

    pub fn get_filechooser(&self, filechooser_name: &str) -> gtk::FileDialog {
        self.builder
            .object(filechooser_name)
            .expect("Couldn't get filechooser")
    }

}