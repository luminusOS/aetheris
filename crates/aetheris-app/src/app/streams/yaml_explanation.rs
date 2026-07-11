use super::super::utils::*;
use super::super::yaml::*;
use super::super::*;

impl App {
    pub(crate) fn show_yaml_explanation(&self, root: &<Self as Component>::Root) {
        let explanation = build_yaml_explanation_content(
            &text_buffer_text(&self.detail.yaml_buffer),
            self.detail.target.as_ref(),
        );
        let dialog = adw::Dialog::builder()
            .title(tr("YAML Explanation"))
            .content_width(640)
            .content_height(620)
            .build();
        let toolbar = adw::ToolbarView::new();
        toolbar.add_top_bar(&adw::HeaderBar::new());

        let scrolled = gtk::ScrolledWindow::builder()
            .vexpand(true)
            .hexpand(true)
            .build();
        scrolled.set_child(Some(&explanation));
        toolbar.set_content(Some(&scrolled));
        dialog.set_child(Some(&toolbar));
        dialog.present(Some(root));
    }
}
