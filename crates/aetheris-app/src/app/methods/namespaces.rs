use super::super::*;

impl App {
    pub(crate) fn show_custom_namespace_dialog(&self, root: &<Self as Component>::Root) {
        self.custom_namespace_entry.set_text("");
        self.custom_namespace_entry.grab_focus();
        self.custom_namespace_dialog.present(Some(root));
    }

    pub(crate) fn open_rename_namespace_dialog(
        &mut self,
        namespace: &str,
        root: &<Self as Component>::Root,
    ) {
        self.renaming_namespace = Some(namespace.to_owned());
        self.rename_namespace_entry.set_text(namespace);
        self.rename_namespace_entry.grab_focus();
        self.rename_namespace_dialog.present(Some(root));
    }

    pub(crate) fn namespace_choices(&self) -> Vec<String> {
        let mut choices = self.namespaces.clone();
        if let Some(project) = self.projects.selected_project() {
            choices.extend(project.custom_namespaces_for_context(self.selected_context.as_deref()));
        }

        if !self.selected_namespace.is_empty()
            && !choices
                .iter()
                .any(|namespace| namespace == &self.selected_namespace)
        {
            choices.push(self.selected_namespace.clone());
        }

        choices.sort();
        choices.dedup();
        if let Some(index) = choices.iter().position(|namespace| namespace == "all") {
            let all = choices.remove(index);
            choices.insert(0, all);
        }
        choices
    }

    pub(crate) fn namespace_is_known(&self, namespace: &str) -> bool {
        self.namespaces.iter().any(|known| known == namespace)
            || self.projects.selected_project().is_some_and(|project| {
                project.has_custom_namespace(self.selected_context.as_deref(), namespace)
            })
    }

    pub(crate) fn preferred_namespace_for_selected_context(&self, fallback: &str) -> String {
        self.projects
            .last_namespace_for_context(self.selected_context.as_deref())
            .filter(|namespace| self.namespace_is_known(namespace))
            .map(str::to_owned)
            .unwrap_or_else(|| {
                self.namespaces
                    .first()
                    .cloned()
                    .unwrap_or_else(|| String::from(fallback))
            })
    }

    pub(crate) fn remember_selected_namespace(&mut self) {
        let Some(context) = self.selected_context.clone() else {
            return;
        };
        if self
            .projects
            .set_last_namespace_for_context(&context, &self.selected_namespace)
        {
            self.save_projects_or_toast();
        }
    }

    pub(crate) fn remember_namespace(&mut self, namespace: &str) {
        if namespace == "all" || namespace.is_empty() {
            return;
        }

        if !self.namespaces.iter().any(|known| known == namespace) {
            self.namespaces.push(namespace.to_owned());
        }

        if let Some(context) = self.selected_context.clone()
            && let Some(project) = self.projects.selected_project_mut()
            && project.add_custom_namespace(&context, namespace)
        {
            self.save_projects_or_toast();
        }
    }
}
