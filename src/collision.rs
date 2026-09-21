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
/// A sphere sweeping past a box meets a shape with flat faces, rounded edges
/// and rounded corners: the box grown by the radius, with its corners filed
/// off. Growing the box and casting a ray at it gets the faces right and the
/// corners wrong, and the difference is not academic. A ball rolling level with
/// the top of a platform toward the next one is nowhere near touching it, but a
/// square corner says otherwise and stops it dead at the seam.
pub fn sweep_sphere(sphere: &Sphere, movement: Vec3, box_: &Aabb) -> Option<Hit> {
    let travel = movement.length();
    if travel < f32::EPSILON {
        return None;
    }

    let grown = box_.expanded(Vec3::splat(sphere.radius));

    // Already in contact, which is what resting on a floor is, and also what
    // sitting at its edge is: the sphere can be clear of the box while its
    // center is still inside the grown one. A ray from in there reports a hit
    // at no distance with whichever axis the slab test looked at first, which
    // would block rolling along a floor, and off the end of it, as surely as
    // falling through it. Decide by direction instead: into the surface is
    // blocked, away from it or along it is free.
    if grown.contains_point(sphere.center) {
        let away = nearest_face(&grown, sphere.center);

        return if movement.dot(away) >= 0.0 {
            None
        } else {
            Some(Hit {
                distance: 0.0,
                point: box_.closest_point(sphere.center),
                normal: away,
            })
        };
    }

    let ray = Ray::new(sphere.center, movement);
    let entry = ray.hit_aabb(&grown)?;
    if entry.distance > travel {
        return None;
    }

    // Which faces of the grown box the entry point sits beyond tells us what
    // the sphere is really about to meet: one axis is a flat face, two is a
    // rounded edge, three is a rounded corner.
    let point = ray.at(entry.distance);
    let mut beyond = [false; 3];
    let mut count = 0;
    for axis in 0..3 {
        if point[axis] < box_.min[axis] || point[axis] > box_.max[axis] {
            beyond[axis] = true;
            count += 1;
        }
    }

    if count <= 1 {
        return Some(Hit {
            distance: entry.distance,
            point,
            normal: entry.normal,
        });
    }

    // The corner or edge the sphere is heading for, as a point or a segment.
    let mut corner = Vec3::ZERO;
    for axis in 0..3 {
        corner[axis] = if point[axis] < box_.min[axis] {
            box_.min[axis]
        } else if point[axis] > box_.max[axis] {
            box_.max[axis]
        } else {
            point[axis]
        };
    }

    let distance = if count == 3 {
        // a corner: the rounded part is a sphere sitting on it
        ray.hit_sphere(&Sphere::new(corner, sphere.radius))
            .map(|hit| hit.distance)
    } else {
        // an edge: the rounded part is a cylinder lying along it
        let free = (0..3).find(|axis| !beyond[*axis]).expect("two axes are beyond");
        sweep_against_edge(&ray, corner, free, box_, sphere.radius)
    }?;

    if distance > travel {
        return None;
    }

    let center = ray.at(distance);
    let normal = (center - box_.closest_point(center)).normalize_or_zero();

    Some(Hit {
        distance,
        point: box_.closest_point(center),
        normal: if normal.length_squared() < 0.5 {
            entry.normal
        } else {
            normal
        },
    })
}

/// How far along `ray` a sphere of `radius` first touches the box edge running
/// along `free` through `corner`.
fn sweep_against_edge(ray: &Ray, corner: Vec3, free: usize, box_: &Aabb, radius: f32) -> Option<f32> {
    // flatten out the axis the edge runs along: what is left is a circle
    let others: Vec<usize> = (0..3).filter(|axis| *axis != free).collect();
    let (a, b) = (others[0], others[1]);

    let ox = ray.origin[a] - corner[a];
    let oy = ray.origin[b] - corner[b];
    let dx = ray.direction[a];
    let dy = ray.direction[b];

    let quadratic_a = dx * dx + dy * dy;
    if quadratic_a < f32::EPSILON {
        // travelling straight along the edge, so it never closes in on it
        return None;
    }

    let half_b = ox * dx + oy * dy;
    let c = ox * ox + oy * oy - radius * radius;
    let discriminant = half_b * half_b - quadratic_a * c;
    if discriminant < 0.0 {
        return None;
    }

    let distance = (-half_b - discriminant.sqrt()) / quadratic_a;
    if distance < 0.0 {
        return None;
    }

    // it only counts if it lands on the edge itself rather than past its ends,
    // where a corner takes over
    let along = ray.origin[free] + ray.direction[free] * distance;
    if along < box_.min[free] || along > box_.max[free] {
        return ray
            .hit_sphere(&Sphere::new(nearest_end(corner, free, box_, along), radius))
            .map(|hit| hit.distance);
    }

    Some(distance)
}

fn nearest_end(corner: Vec3, free: usize, box_: &Aabb, along: f32) -> Vec3 {
    let mut end = corner;
    end[free] = if along < box_.min[free] {
        box_.min[free]
    } else {
        box_.max[free]
    };
    end
}

/// Which way out of a box a point is nearest to, as an outward normal.
///
/// This is the contact normal for a sphere already touching a box, taken from
/// the box grown by its radius. Taking it from the nearest point on the box
/// itself looks reasonable and is wrong: a ball resting level with the top of a
/// box reads as pressed into its side, so rolling from one platform onto
/// another at the same height gets blocked at the seam.
fn nearest_face(box_: &Aabb, point: Vec3) -> Vec3 {
    let to_min = point - box_.min;
    let to_max = box_.max - point;

    let mut normal = Vec3::Y;
    let mut nearest = f32::INFINITY;

    for axis in 0..3 {
        if to_min[axis] < nearest {
            nearest = to_min[axis];
            normal = Vec3::ZERO;
            normal[axis] = -1.0;
        }
        if to_max[axis] < nearest {
            nearest = to_max[axis];
            normal = Vec3::ZERO;
            normal[axis] = 1.0;
        }
    }

    normal
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
    fn resting_on_a_floor_rolls_but_does_not_sink() {
        let floor = Aabb::from_center_size(vec3(0.0, -0.5, 0.0), vec3(20.0, 1.0, 20.0));
        // exactly touching, which is where a ball at rest ends up
        let ball = Sphere::new(vec3(0.0, 0.5, 0.0), 0.5);

        // along the floor is free
        assert!(sweep_sphere(&ball, vec3(1.0, 0.0, 0.0), &floor).is_none());
        // up and away is free
        assert!(sweep_sphere(&ball, vec3(0.0, 1.0, 0.0), &floor).is_none());
        // down into it is not
        let hit = sweep_sphere(&ball, vec3(0.0, -1.0, 0.0), &floor)
            .expect("gravity should still be stopped by the floor");
        assert_eq!(hit.distance, 0.0);
        assert!((hit.normal - Vec3::Y).length() < 1e-5, "{:?}", hit.normal);

        // and rolling along it keeps its whole step
        let end = move_and_slide(ball, vec3(4.0, 0.0, 0.0), 1.0, &[floor]);
        assert!((end.x - 4.0).abs() < 1e-3, "rolled to {:?}", end);
    }

    #[test]
    fn a_ball_rolls_off_the_end_of_a_floor() {
        // the floor runs to x = 10, and the ball is just past its edge: clear of
        // the box, but still inside the box grown by its radius
        let floor = Aabb::from_center_size(vec3(0.0, -0.5, 0.0), vec3(20.0, 1.0, 20.0));
        let ball = Sphere::new(vec3(10.14, 0.5, 0.0), 0.5);

        assert!(!ball.intersects_aabb(&floor), "the ball is past the edge");

        // carrying on outward is free, not blocked by the floor behind it
        assert!(sweep_sphere(&ball, vec3(1.0, 0.0, 0.0), &floor).is_none());
        let end = move_and_slide(ball, vec3(4.0, 0.0, 0.0), 1.0, &[floor]);
        assert!(end.x > 13.0, "it stopped at the edge: {:?}", end);
    }

    #[test]
    fn a_ball_rolls_from_one_platform_onto_the_next() {
        // two platforms whose tops are level, meeting at z = 0
        let first = Aabb::from_center_size(vec3(0.0, -0.5, 4.0), vec3(10.0, 1.0, 10.0));
        let second = Aabb::from_center_size(vec3(0.0, -0.5, -6.0), vec3(5.0, 1.0, 12.0));
        let ball = Sphere::new(vec3(0.0, 0.5, 0.5), 0.5);

        // rolling toward the seam is not rolling into a wall
        assert!(sweep_sphere(&ball, vec3(0.0, 0.0, -2.0), &second).is_none());

        let end = move_and_slide(ball, vec3(0.0, 0.0, -6.0), 1.0, &[first, second]);
        assert!(end.z < -4.0, "it stopped at the seam: {:?}", end);
    }

    #[test]
    fn a_ball_level_with_a_platform_top_is_not_blocked_by_its_side() {
        // the exact case the marble game hit: rolling along one platform toward
        // another whose top is at the same height. The ball's center is 0.57
        // from the platform, well clear of its 0.4 radius, so nothing should
        // stop it, though a square cornered sweep says otherwise.
        let next = Aabb::from_center_size(vec3(0.0, -0.5, -6.0), vec3(5.0, 1.0, 12.0));
        let ball = Sphere::new(vec3(0.0, 0.4, 0.6), 0.4);

        assert!(
            sweep_sphere(&ball, vec3(0.0, 0.0, -0.5), &next).is_none(),
            "stopped short of a platform it is level with"
        );

        let end = move_and_slide(ball, vec3(0.0, 0.0, -6.0), 1.0, &[next]);
        assert!(end.z < -4.0, "it stopped at the seam: {:?}", end);
    }

    #[test]
    fn a_ball_still_lands_on_a_platform_from_above() {
        let platform = Aabb::from_center_size(vec3(0.0, -0.5, 0.0), vec3(10.0, 1.0, 10.0));
        let ball = Sphere::new(vec3(0.0, 4.0, 0.0), 0.4);

        let hit = sweep_sphere(&ball, vec3(0.0, -8.0, 0.0), &platform)
            .expect("falling onto a platform still stops");

        assert!((hit.distance - 3.6).abs() < 1e-3, "distance {}", hit.distance);
        assert!((hit.normal - Vec3::Y).length() < 1e-3, "{:?}", hit.normal);
    }

    #[test]
    fn a_ball_is_still_stopped_by_a_wall_it_faces() {
        let wall = Aabb::from_center_size(vec3(3.0, 1.0, 0.0), vec3(1.0, 2.0, 10.0));
        let ball = Sphere::new(vec3(0.0, 0.5, 0.0), 0.4);

        let hit = sweep_sphere(&ball, vec3(6.0, 0.0, 0.0), &wall).expect("a wall stops it");

        // the wall's near face is at x 2.5, so contact is 0.4 short of it
        assert!((hit.distance - 2.1).abs() < 1e-3, "distance {}", hit.distance);
        assert!((hit.normal - vec3(-1.0, 0.0, 0.0)).length() < 1e-3, "{:?}", hit.normal);
    }

    #[test]
    fn a_corner_is_rounded_rather_than_square() {
        let box_ = Aabb::from_center_size(Vec3::ZERO, Vec3::splat(2.0));
        // heading at the corner diagonally from outside
        let ball = Sphere::new(vec3(3.0, 3.0, 0.0), 0.5);

        let hit = sweep_sphere(&ball, vec3(-4.0, -4.0, 0.0), &box_).expect("it meets the corner");

        // a square corner would stop it at the grown box, 0.5 further out than
        // the rounded one, which sits 0.5 from the corner along the diagonal
        let center = vec3(3.0, 3.0, 0.0) + vec3(-4.0, -4.0, 0.0).normalize() * hit.distance;
        let corner = vec3(1.0, 1.0, 0.0);
        assert!(
            ((center - corner).length() - 0.5).abs() < 1e-3,
            "contact was {} from the corner",
            (center - corner).length()
        );
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
