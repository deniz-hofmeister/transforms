use ros2_client::rclrs_common::QosProfile;
use ros2_client::{Context, Node, Subscription};
use std::sync::Arc;
use tf2_msgs::msg::TFMessage;
use tokio::sync::Mutex;

struct TransformWrapper {
    registry: Arc<Mutex<Registry>>,
}

impl TransformWrapper {
    async fn new() -> Self {
        let context = Context::new().unwrap();
        let node = Node::new(&context, "transform_listener", "").unwrap();

        let registry = Arc::new(Mutex::new(Registry::new(f64::INFINITY)));

        let registry_clone = Arc::clone(&registry);
        let _subscription = node
            .create_subscription::<TFMessage>(
                "/tf",
                QosProfile::default(),
                move |msg: TFMessage| {
                    let mut registry = registry_clone.lock().await;
                    for transform in msg.transforms {
                        let custom_transform = Transform {
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
                            parent: transform.header.frame_id.clone(),
                            child: transform.child_frame_id.clone(),
                        };
                        registry.add_transform(custom_transform).unwrap();
                    }
                },
            )
            .unwrap();

        TransformWrapper { registry }
    }
}

#[tokio::main]
async fn main() {
    let _wrapper = TransformWrapper::new().await;
    // Keep the node alive
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
}
