# Transforms

## Coordinate frames transforms through time and space

### Features
Transforms offers the ability to transform the coordinates of a 3D point with timestamp, from one reference frame to another.

### Background
The robotics field needs coordinate frame transforms, for example for lidar data, GPS data and many other sources. On top of that, it usually has tight performance constraints. Combine these two and one quickly would need a performant coordinates transform such that data can be processed from the same reference frame, most often at the robot's center of gravity. 

This library draws inspiration from ROS2 and the TF2 package. 

### Requirements
It is a requirement that the point's timestamp falls within or exactly on the timestamps of known reference frames. For instance, given two reference frames R1 and R2 with known transforms at T=0 and T=10 each, then transformations are possible for points with timestamps in the range T=[0,10], inclusive. Linear interpolation is used for timestamps between known transforms. This approach ensures accurate transformations within the defined time range.

### Limitations
There are a few constraints that this system adheres to. Primarily to keep the system precise and minimal.

#### Linear Interpolation Only
This library will only linearly interpolate between coordinate frames and timestamps. There will be no polynomial curve fitting or any other forms of more advanced estimation methods. This is to keep the system simple and performant. If you want more precise transforms then feel free to register more transforms with a finer delta time. After all, the inaccuracies only arise when there are large time gaps between known transforms.

#### No Extrapolation
Extrapolation is potentially dangerous and/or unstable. One can argue that linear extrapolation or zero-order-hold extrapolation is still acceptable and perfectly fine to use. This system refuses to do it as it opens up the discussion of "How far of extrapolation should we allow?", "Let the user configure extrapolation..." and many other edge-case discussions that lead to unessecary complexity, discussion and behavior unpredicability. If you really want extrapolation, then you are free to publish a transform into the future and let this library "interpolate" within that.

#### Once Reference Frames have been assigned children and/or parents, their relationships cannot be modified
At the moment there is no intent for dynamic re-assigning of reference frames and transforms.

#### Assume Reference Frames will live on for the entire lifetime of the process
With exception of reference frames timing out in the buffer. When this happens, the transforms are lost.

#### Reference Frame names have to be unique
If you run into an issue regarding this, consider using prefixes to group your reference frames

#### No 1-to-1 equivelance to ROS2's TF2 library
This library is inspired by the TF2 library but makes no promise of matching its features, API or implementation. It's by inspiration only.

### Roadmap

v0 Exploration of the domain, provide functionality, built for purpose
v1 Refactor for performance
v2 Everything else...


### Note
Everything said above is up for change
