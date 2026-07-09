use anyhow::{Context as AnyhowContext, Result};
use k8s_openapi::api::core::v1::Event;
use k8s_openapi::jiff::Timestamp;
use kube::api::ListParams;
use kube::{Api, ResourceExt};

use crate::status::age_label;
use crate::{KubeSession, ObjectEvent, ResourceKind, resource_scope};

impl KubeSession {
    pub(crate) async fn object_events(
        &self,
        resource: &ResourceKind,
        namespace: &str,
        name: &str,
    ) -> Result<Vec<ObjectEvent>> {
        let events: Api<Event> = if namespace.is_empty() || namespace == "-" {
            Api::all(self.client.clone())
        } else {
            Api::namespaced(self.client.clone(), namespace)
        };
        let field_selector = format!(
            "involvedObject.name={name},involvedObject.kind={}",
            resource.kind
        );
        let params = ListParams::default().fields(&field_selector);
        let mut items = events
            .list(&params)
            .await
            .with_context(|| {
                format!(
                    "Could not list Events for {} {name} {}.",
                    resource.kind,
                    resource_scope(resource, Some(namespace))
                )
            })?
            .items;

        items.sort_by(|left, right| {
            event_timestamp(right)
                .cmp(&event_timestamp(left))
                .then_with(|| left.name_any().cmp(&right.name_any()))
        });

        Ok(items.into_iter().map(object_event).collect())
    }
}

fn object_event(event: Event) -> ObjectEvent {
    let last_seen = event_timestamp(&event)
        .map(age_label)
        .unwrap_or_else(|| String::from("-"));
    let fallback_name = event.name_any();

    ObjectEvent {
        type_: event.type_.unwrap_or_else(|| String::from("-")),
        reason: event.reason.unwrap_or(fallback_name),
        message: event.message.unwrap_or_else(|| String::from("-")),
        count: event
            .count
            .map(|count| count.to_string())
            .unwrap_or_else(|| String::from("-")),
        last_seen,
    }
}

fn event_timestamp(event: &Event) -> Option<Timestamp> {
    event
        .last_timestamp
        .as_ref()
        .map(|timestamp| timestamp.0)
        .or_else(|| event.event_time.as_ref().map(|timestamp| timestamp.0))
        .or_else(|| event.first_timestamp.as_ref().map(|timestamp| timestamp.0))
        .or_else(|| {
            event
                .metadata
                .creation_timestamp
                .as_ref()
                .map(|timestamp| timestamp.0)
        })
}
