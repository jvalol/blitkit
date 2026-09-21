//! Shapes that can touch, and the one piece of movement the engine provides.
//!
//! See `specs/0014-collision.md`. Pure maths on the CPU: no GPU, no rigid
//! bodies, no solver. A game decides what a collision means.

use glam::Vec3;

/// How far a slide stops short of a surface, so it does not end up inside it
/// and stick there.
const SKIN: f32 = 1e-3;

/// A box aligned to the world axes.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    /// Takes the corners in any order.
    pub fn new(a: Vec3, b: Vec3) -> Self {
        Self {
            min: a.min(b),
            max: a.max(b),
        }
    }

    pub fn from_center_size(center: Vec3, size: Vec3) -> Self {
        let half = size.abs() * 0.5;
        Self {
            min: center - half,
            max: center + half,
        }
    }

    /// An empty box that grows to fit whatever is added to it.
    pub fn empty() -> Self {
        Self {
            min: Vec3::splat(f32::INFINITY),
            max: Vec3::splat(f32::NEG_INFINITY),
        }
    }

    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    pub fn size(&self) -> Vec3 {
        (self.max - self.min).max(Vec3::ZERO)
    }

    pub fn contains_point(&self, point: Vec3) -> bool {
        point.cmpge(self.min).all() && point.cmple(self.max).all()
    }

    /// Touching exactly counts as overlapping, so a ball resting on the floor
    /// is in contact with it.
    pub fn intersects(&self, other: &Aabb) -> bool {
        self.min.cmple(other.max).all() && self.max.cmpge(other.min).all()
    }

    /// The point of this box nearest to `point`, which is the point itself when
    /// it is inside.
    pub fn closest_point(&self, point: Vec3) -> Vec3 {
        point.clamp(self.min, self.max)
    }

    pub fn expanded(&self, amount: Vec3) -> Self {
        Self {
            min: self.min - amount,
            max: self.max + amount,
        }
    }

    pub fn union_point(&self, point: Vec3) -> Self {
        Self {
            min: self.min.min(point),
            max: self.max.max(point),
        }
    }

    pub fn union(&self, other: &Aabb) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Sphere {
    pub center: Vec3,
    pub radius: f32,
}

impl Sphere {
    pub fn new(center: Vec3, radius: f32) -> Self {
        Self {
            center,
            radius: radius.max(0.0),
        }
    }

    pub fn contains_point(&self, point: Vec3) -> bool {
        (point - self.center).length_squared() <= self.radius * self.radius
    }

    pub fn intersects(&self, other: &Sphere) -> bool {
        let reach = self.radius + other.radius;
        (other.center - self.center).length_squared() <= reach * reach
    }

    /// True when the box's nearest point is within reach, which covers a face,
    /// an edge and a corner alike.
    pub fn intersects_aabb(&self, box_: &Aabb) -> bool {
        let closest = box_.closest_point(self.center);
        (closest - self.center).length_squared() <= self.radius * self.radius
    }
}

/// Where something was hit, how far along, and which way the surface faces.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Hit {
    pub distance: f32,
    pub point: Vec3,
    pub normal: Vec3,
}

#[derive(Debug, Copy, Clone)]
pub struct Ray {
    pub origin: Vec3,
    /// Always a unit vector.
    pub direction: Vec3,
}

impl Ray {
    pub fn new(origin: Vec3, direction: Vec3) -> Self {
        Self {
            origin,
            direction: direction.normalize_or_zero(),
        }
    }

    pub fn at(&self, distance: f32) -> Vec3 {
        self.origin + self.direction * distance
    }

    /// The slab method: clip the ray against each pair of parallel faces and
    /// see whether anything is left.
    pub fn hit_aabb(&self, box_: &Aabb) -> Option<Hit> {
        let mut near = 0.0f32;
        let mut far = f32::INFINITY;
        let mut axis = 0;

        for i in 0..3 {
            let direction = self.direction[i];
            let origin = self.origin[i];

            if direction.abs() < f32::EPSILON {
                // parallel to this pair of faces, so it has to start between them
                if origin < box_.min[i] || origin > box_.max[i] {
                    return None;
                }
                continue;
            }

            let mut t1 = (box_.min[i] - origin) / direction;
            let mut t2 = (box_.max[i] - origin) / direction;
            if t1 > t2 {
                std::mem::swap(&mut t1, &mut t2);
            }

            if t1 > near {
                near = t1;
                axis = i;
            }
            far = far.min(t2);

            if near > far {
                return None;
            }
        }

        let mut normal = Vec3::ZERO;
        normal[axis] = if self.direction[axis] < 0.0 { 1.0 } else { -1.0 };

        Some(Hit {
            distance: near,
            point: self.at(near),
            normal,
        })
    }

    pub fn hit_sphere(&self, sphere: &Sphere) -> Option<Hit> {
        let to_center = self.origin - sphere.center;
        let b = to_center.dot(self.direction);
        let c = to_center.length_squared() - sphere.radius * sphere.radius;

        // starting outside and pointing away
        if c > 0.0 && b > 0.0 {
            return None;
        }

        let discriminant = b * b - c;
        if discriminant < 0.0 {
            return None;
        }

        let distance = (-b - discriminant.sqrt()).max(0.0);
        let point = self.at(distance);

        Some(Hit {
            distance,
            point,
            normal: (point - sphere.center).normalize_or_zero(),
        })
    }
}

/// Moves a sphere along `movement` and reports the first box it touches.
///
/// The box is grown by the sphere's radius and a ray is cast at it, which
/// treats the sphere as square at the corners. A ball clipping the very corner
/// of a box stops slightly early, which is the forgiving direction.
pub fn sweep_sphere(sphere: &Sphere, movement: Vec3, box_: &Aabb) -> Option<Hit> {
    let distance = movement.length();
    if distance < f32::EPSILON {
        return None;
    }

    let grown = box_.expanded(Vec3::splat(sphere.radius));
    let ray = Ray::new(sphere.center, movement);

    match ray.hit_aabb(&grown) {
        Some(hit) if hit.distance <= distance => Some(hit),
        _ => None,
    }
}

/// Moves a sphere by `velocity` for `dt`, sliding along whatever it hits rather
/// than stopping dead, and returns where it ends up.
///
/// It gives up after four surfaces, so a corner settles instead of ringing
/// between two walls forever.
pub fn move_and_slide(sphere: Sphere, velocity: Vec3, dt: f32, colliders: &[Aabb]) -> Vec3 {
    let mut position = sphere.center;
    let mut remaining = velocity * dt;

    for _ in 0..4 {
        if remaining.length_squared() < f32::EPSILON {
            break;
        }

        let moving = Sphere::new(position, sphere.radius);
        let hit = colliders
            .iter()
            .filter_map(|box_| sweep_sphere(&moving, remaining, box_))
            .min_by(|a, b| a.distance.total_cmp(&b.distance));

        let hit = match hit {
            Some(hit) => hit,
            None => {
                position += remaining;
                break;
            }
        };

        // stop just short of the surface, then carry the rest of the movement
        // along it rather than into it
        let travelled = (hit.distance - SKIN).max(0.0);
        let direction = remaining.normalize_or_zero();
        position += direction * travelled;

        let left = remaining.length() - travelled;
        let along = direction - hit.normal * direction.dot(hit.normal);
        remaining = along * left;
    }

    position
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::vec3;

    fn unit_box() -> Aabb {
        Aabb::from_center_size(Vec3::ZERO, Vec3::splat(2.0))
    }

    #[test]
    fn boxes_overlap_or_miss() {
        let box_ = unit_box();

        assert!(box_.intersects(&Aabb::from_center_size(vec3(1.5, 0.0, 0.0), Vec3::ONE)));
        assert!(!box_.intersects(&Aabb::from_center_size(vec3(3.0, 0.0, 0.0), Vec3::ONE)));
        // touching exactly counts, so a ball resting on a floor is in contact
        assert!(box_.intersects(&Aabb::from_center_size(vec3(2.0, 0.0, 0.0), Vec3::splat(2.0))));
    }

    #[test]
    fn spheres_overlap_or_miss() {
        let sphere = Sphere::new(Vec3::ZERO, 1.0);

        assert!(sphere.intersects(&Sphere::new(vec3(1.5, 0.0, 0.0), 1.0)));
        assert!(!sphere.intersects(&Sphere::new(vec3(2.5, 0.0, 0.0), 1.0)));
        assert!(sphere.intersects(&Sphere::new(vec3(2.0, 0.0, 0.0), 1.0)));
    }

    #[test]
    fn a_sphere_meets_a_box() {
        let box_ = unit_box();

        // through a face
        assert!(Sphere::new(vec3(1.5, 0.0, 0.0), 0.6).intersects_aabb(&box_));
        assert!(!Sphere::new(vec3(1.5, 0.0, 0.0), 0.4).intersects_aabb(&box_));

        // at an edge, where the nearest point is on two faces at once
        assert!(Sphere::new(vec3(1.3, 1.3, 0.0), 0.5).intersects_aabb(&box_));
        assert!(!Sphere::new(vec3(1.6, 1.6, 0.0), 0.5).intersects_aabb(&box_));

        // at a corner, the hardest case for a naive test
        assert!(Sphere::new(vec3(1.2, 1.2, 1.2), 0.5).intersects_aabb(&box_));
        assert!(!Sphere::new(vec3(1.5, 1.5, 1.5), 0.5).intersects_aabb(&box_));
    }

    #[test]
    fn points_are_inside_or_outside() {
        let box_ = unit_box();
        assert!(box_.contains_point(Vec3::ZERO));
        assert!(box_.contains_point(vec3(1.0, 1.0, 1.0)));
        assert!(!box_.contains_point(vec3(1.1, 0.0, 0.0)));

        let sphere = Sphere::new(Vec3::ZERO, 1.0);
        assert!(sphere.contains_point(vec3(0.5, 0.5, 0.0)));
        assert!(!sphere.contains_point(vec3(0.9, 0.9, 0.0)));
    }

    #[test]
    fn a_ray_hits_a_box() {
        let hit = Ray::new(vec3(-5.0, 0.0, 0.0), Vec3::X)
            .hit_aabb(&unit_box())
            .expect("the ray points straight at it");

        assert!((hit.distance - 4.0).abs() < 1e-5, "distance {}", hit.distance);
        assert!((hit.point - vec3(-1.0, 0.0, 0.0)).length() < 1e-5);
        // the face it struck looks back along the ray
        assert!((hit.normal - vec3(-1.0, 0.0, 0.0)).length() < 1e-5);
    }

    #[test]
    fn a_ray_pointing_away_misses() {
        let ray = Ray::new(vec3(-5.0, 0.0, 0.0), -Vec3::X);

        assert!(ray.hit_aabb(&unit_box()).is_none());
        assert!(ray.hit_sphere(&Sphere::new(Vec3::ZERO, 1.0)).is_none());
    }

    #[test]
    fn a_ray_hits_a_sphere() {
        let hit = Ray::new(vec3(0.0, 0.0, 5.0), -Vec3::Z)
            .hit_sphere(&Sphere::new(Vec3::ZERO, 2.0))
            .expect("the ray points straight at it");

        // the near surface, not the far one
        assert!((hit.distance - 3.0).abs() < 1e-5, "distance {}", hit.distance);
        assert!((hit.normal - Vec3::Z).length() < 1e-5);
    }

    #[test]
    fn a_swept_sphere_does_not_tunnel() {
        let wall = Aabb::from_center_size(vec3(5.0, 0.0, 0.0), vec3(0.2, 10.0, 10.0));
        let ball = Sphere::new(Vec3::ZERO, 0.5);

        // a step that would jump clean past a thin wall
        let hit = sweep_sphere(&ball, vec3(20.0, 0.0, 0.0), &wall)
            .expect("the sweep catches what a snapshot would miss");

        assert!(hit.distance < 5.0, "stopped at {}", hit.distance);
        // and a test of where it ended up would have seen nothing
        assert!(!Sphere::new(vec3(20.0, 0.0, 0.0), 0.5).intersects_aabb(&wall));
    }

    #[test]
    fn move_and_slide_slides() {
        let wall = Aabb::from_center_size(vec3(2.0, 0.0, 0.0), vec3(1.0, 10.0, 10.0));
        let ball = Sphere::new(Vec3::ZERO, 0.5);

        // driven diagonally into a wall that blocks x but not z
        let end = move_and_slide(ball, vec3(4.0, 0.0, 4.0), 1.0, &[wall]);

        assert!(end.x < 1.1, "should be stopped by the wall, x was {}", end.x);
        assert!(end.z > 1.0, "should have slid along it, z was {}", end.z);
    }

    #[test]
    fn move_and_slide_settles_in_a_corner() {
        let walls = [
            Aabb::from_center_size(vec3(2.0, 0.0, 0.0), vec3(1.0, 10.0, 10.0)),
            Aabb::from_center_size(vec3(0.0, 0.0, 2.0), vec3(10.0, 10.0, 1.0)),
        ];
        let ball = Sphere::new(Vec3::ZERO, 0.5);

        let end = move_and_slide(ball, vec3(4.0, 0.0, 4.0), 1.0, &walls);

        // wedged, not shot through either wall and not flung away
        assert!(end.x < 1.1 && end.z < 1.1, "ended at {:?}", end);
        assert!(end.x > -0.1 && end.z > -0.1, "ended at {:?}", end);
        for wall in walls.iter() {
            assert!(!Sphere::new(end, 0.5).intersects_aabb(wall), "inside a wall at {:?}", end);
        }
    }

    #[test]
    fn move_and_slide_leaves_open_space_alone() {
        let far_away = Aabb::from_center_size(vec3(50.0, 0.0, 0.0), Vec3::ONE);
        let ball = Sphere::new(Vec3::ZERO, 0.5);

        let end = move_and_slide(ball, vec3(1.0, 2.0, 3.0), 0.5, &[far_away]);

        assert!((end - vec3(0.5, 1.0, 1.5)).length() < 1e-5, "ended at {:?}", end);
    }
}
