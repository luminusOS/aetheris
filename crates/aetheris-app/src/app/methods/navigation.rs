use super::super::*;

impl App {
    pub(crate) fn show_projects(&self) {
        self.root_stack.set_visible_child_name("projects");
    }

    pub(crate) fn show_browser(&self) {
        self.root_stack.set_visible_child_name("browser");
    }

    pub(crate) fn enter_clusters_page(&mut self, sender: ComponentSender<Self>) {
        self.rebuild_cluster_list();
        self.ensure_cluster_summaries_loading(sender);
        self.show_clusters();
    }

    pub(crate) fn show_clusters(&self) {
        self.root_stack.set_visible_child_name("clusters");
    }

    pub(crate) fn switch_to_project(&mut self, sender: ComponentSender<Self>) {
        if !self
            .visible_contexts()
            .iter()
            .any(|context| self.selected_context.as_deref() == Some(context.name.as_str()))
        {
            self.selected_context = None;
        }
        self.sync_dropdowns(Some(sender.clone()));
        self.enter_clusters_page(sender);
        self.present_content_panel();
        self.loading = false;
        self.status = tr("Select a cluster.");
        self.sync_status();
    }

    pub(crate) fn show_object_list(&self) {
        self.content_stack.set_visible_child_name("list");
        self.content_header_stack.set_visible_child_name("search");
        self.detail.back_button.set_visible(false);
        self.detail.delete_button.set_visible(false);
        self.detail.favorite_button.set_visible(false);
        self.detail.terminal_button.set_visible(false);
    }

    pub(crate) fn show_detail_page(&self, title: &str) {
        self.content_stack.set_visible_child_name("detail");
        self.content_title_label.set_label(title);
        self.content_header_stack.set_visible_child_name("title");
        self.detail.back_button.set_visible(true);
        self.detail.delete_button.set_visible(true);
        self.detail.favorite_button.set_visible(true);
        self.sync_detail_favorite_button();
        self.sync_terminal_controls();
    }

    // Nautilus behavior: picking something in the overlay sidebar dismisses
    // it so the content it drives is immediately visible; when the sidebar
    // sits side-by-side there is nothing to dismiss.
    pub(crate) fn present_content_panel(&self) {
        if self.split_view.is_collapsed() {
            self.split_view.set_show_sidebar(false);
        }
    }
}
