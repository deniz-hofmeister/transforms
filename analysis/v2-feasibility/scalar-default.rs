//! Minimal Rust type-inference witness, not an implementation of f32 transforms.

#[cfg(not(generic))]
struct Vector(f64);

#[cfg(not(generic))]
impl Vector {
    fn zero() -> Self {
        Self(0.0)
    }
}

#[cfg(generic)]
struct Vector<S = f64>(S);

#[cfg(generic)]
trait Scalar: Default {}

#[cfg(generic)]
impl Scalar for f64 {}

#[cfg(generic)]
impl Scalar for f32 {}

#[cfg(generic)]
impl<S: Scalar> Vector<S> {
    fn zero() -> Self {
        Self(S::default())
    }
}

fn main() {
    let value = Vector::zero();
    let _ = value.0;
}
