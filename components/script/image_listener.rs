/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use crate::dom::bindings::conversions::DerivedFrom;
use crate::dom::bindings::refcounted::Trusted;
use crate::dom::bindings::reflector::DomObject;
use crate::dom::node::{window_from_node, Node};
use crate::task_source::TaskSource;
use rr_channel::ipc;
use rr_channel::router::ROUTER;
use net_traits::image_cache::{ImageCache, ImageResponder, ImageResponse, PendingImageId, PendingImageResponse};
use std::sync::Arc;

pub trait ImageCacheListener {
    fn generation_id(&self) -> u32;
    fn process_image_response(&self, response: ImageResponse);
}

pub fn add_cache_listener_for_element<T: ImageCacheListener + DerivedFrom<Node> + DomObject>(
    image_cache: Arc<dyn ImageCache>,
    id: PendingImageId,
    elem: &T,
) {
    let trusted_node = Trusted::new(elem);
    let (responder_sender, responder_receiver) =
        ipc::channel::<PendingImageResponse>().unwrap();

    let window = window_from_node(elem);
    let (task_source, canceller) = window
        .task_manager()
        .networking_task_source_with_canceller();
    let generation = elem.generation_id();
    ROUTER.add_route(
        responder_receiver,
        Box::new(move |message| {
            let element = trusted_node.clone();
            let image = message.unwrap().response;

            let _ = task_source.queue_with_canceller(
                task!(process_image_response: move || {
                    let element = element.root();
                    // Ignore any image response for a previous request that has been discarded.
                    if generation == element.generation_id() {
                        element.process_image_response(image);
                    }
                }),
                &canceller,
            );
        }),
    );

    image_cache.add_listener(id, ImageResponder::new(responder_sender, id));
}
