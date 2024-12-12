use crate::{errors::TransformError, geometry::Transform};

pub trait Transformable {
    fn transform(
        &mut self,
        transform: &Transform,
    ) -> Result<(), TransformError>;
}
