use super::super::*;

pub(super) struct DetailSignalWidgets<'a> {
    pub(super) back_button: &'a gtk::Button,
    pub(super) stack: &'a gtk::Stack,
    pub(super) apply_button: &'a gtk::Button,
    pub(super) explain_yaml_button: &'a gtk::Button,
    pub(super) download_yaml_button: &'a gtk::Button,
    pub(super) delete_button: &'a gtk::Button,
    pub(super) favorite_button: &'a gtk::Button,
    pub(super) terminal_button: &'a gtk::Button,
    pub(super) scale_button: &'a gtk::Button,
    pub(super) cordon_button: &'a gtk::Button,
    pub(super) drain_button: &'a gtk::Button,
    pub(super) log_start_button: &'a gtk::Button,
    pub(super) log_stop_button: &'a gtk::Button,
    pub(super) log_clear_button: &'a gtk::Button,
    pub(super) expand_logs_button: &'a gtk::Button,
    pub(super) log_download_button: &'a gtk::Button,
    pub(super) port_start_button: &'a gtk::Button,
    pub(super) port_stop_button: &'a gtk::Button,
}

/// Wires every detail-page button/control to its `AppMsg`. Extracted from
/// `init` as a self-contained group: none of these connections read or
/// share state with each other, they only forward clicks to `sender`.
pub(super) fn connect_detail_signals(
    widgets: DetailSignalWidgets<'_>,
    sender: &ComponentSender<App>,
) {
    widgets.back_button.connect_clicked({
        let sender = sender.clone();
        move |_| sender.input(AppMsg::BackToObjects)
    });
    widgets.stack.connect_visible_child_name_notify({
        let sender = sender.clone();
        move |stack| {
            if let Some(name) = stack.visible_child_name() {
                sender.input(AppMsg::DetailTabChanged(name.to_string()));
            }
        }
    });
    widgets.apply_button.connect_clicked({
        let sender = sender.clone();
        move |_| sender.input(AppMsg::ApplyYaml)
    });
    widgets.explain_yaml_button.connect_clicked({
        let sender = sender.clone();
        move |_| sender.input(AppMsg::ExplainYaml)
    });
    widgets.download_yaml_button.connect_clicked({
        let sender = sender.clone();
        move |_| sender.input(AppMsg::DownloadYaml)
    });
    widgets.delete_button.connect_clicked({
        let sender = sender.clone();
        move |_| sender.input(AppMsg::DeleteObject)
    });
    widgets.favorite_button.connect_clicked({
        let sender = sender.clone();
        move |_| sender.input(AppMsg::ToggleCurrentObjectFavorite)
    });
    widgets.terminal_button.connect_clicked({
        let sender = sender.clone();
        move |_| sender.input(AppMsg::ShowPodTerminal)
    });
    widgets.scale_button.connect_clicked({
        let sender = sender.clone();
        move |_| sender.input(AppMsg::ScaleDeployment)
    });
    widgets.cordon_button.connect_clicked({
        let sender = sender.clone();
        move |_| sender.input(AppMsg::ToggleNodeScheduling)
    });
    widgets.drain_button.connect_clicked({
        let sender = sender.clone();
        move |_| sender.input(AppMsg::DrainNode)
    });
    widgets.log_start_button.connect_clicked({
        let sender = sender.clone();
        move |_| sender.input(AppMsg::StartPodLogs)
    });
    widgets.log_stop_button.connect_clicked({
        let sender = sender.clone();
        move |_| sender.input(AppMsg::StopPodLogs)
    });
    widgets.log_clear_button.connect_clicked({
        let sender = sender.clone();
        move |_| sender.input(AppMsg::ClearPodLogs)
    });
    widgets.expand_logs_button.connect_clicked({
        let sender = sender.clone();
        move |_| sender.input(AppMsg::ToggleDetailOverview)
    });
    widgets.log_download_button.connect_clicked({
        let sender = sender.clone();
        move |_| sender.input(AppMsg::DownloadLogs)
    });
    widgets.port_start_button.connect_clicked({
        let sender = sender.clone();
        move |_| sender.input(AppMsg::StartPodPortForward)
    });
    widgets.port_stop_button.connect_clicked({
        let sender = sender.clone();
        move |_| sender.input(AppMsg::StopPodPortForward)
    });
}
