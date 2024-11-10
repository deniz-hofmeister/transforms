use crate::types::{Registry, Transform};
use r2r::QosProfile;

pub struct Tf2Bridge {
    node: r2r::Node,
    registry: Registry,
}

impl Tf2Bridge {
    pub fn new(context: r2r::Context) -> Result<Self, r2r::Error> {
        let node = r2r::Node::create(context, "tf2_bridge", "transforms")?;
        let registry = Registry::new(f64::INFINITY);

        Ok(Self { node, registry })
    }

    pub fn start_listening(&mut self) -> Result<(), r2r::Error> {
        let subscriber = self
            .node
            .subscribe::<r2r::tf2_msgs::msg::TFMessage>("/tf", QosProfile::default())?;

        // Convert and add transforms to registry
        // This is a basic example - you'll need to implement the actual conversion
        subscriber.for_each(|msg| {
            for transform in msg.transforms {
                // Convert tf2 message to your Transform type
                let t = Transform {
                    translation: Vector3 {
                        x: transform.transform.translation.x,
                        y: transform.transform.translation.y,
                        z: transform.transform.translation.z,
                    },
                    rotation: Quaternion {
                        w: transform.transform.rotation.w,
                        x: transform.transform.rotation.x,
                        y: transform.transform.rotation.y,
                        z: transform.transform.rotation.z,
                    },
                    timestamp: Timestamp {
                        nanoseconds: transform.header.stamp.sec as u64 * 1_000_000_000
                            + transform.header.stamp.nanosec as u64,
                    },
                    parent: transform.header.frame_id,
                    child: transform.child_frame_id,
                };

                self.registry.add_transform(t).unwrap_or_else(|e| {
                    log::error!("Failed to add transform: {}", e);
                });
            }
            futures::future::ready(())
        });

        Ok(())
    }

    pub fn get_registry(&self) -> &Registry {
        &self.registry
    }
}
