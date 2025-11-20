// This file is part of Luola2
// Copyright (C) 2025 Calle Laakkonen
//
// Luola2 is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// Luola2 is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Luola2.  If not, see <https://www.gnu.org/licenses/>.

use std::{marker::PhantomData, ptr::NonNull, slice};

use crate::math::Vec2;

/**
 * A sorted array of game objects
 */
pub struct GameObjectArray<T>(Vec<T>);

pub trait GameObject {
    /// Return the center of this object
    fn pos(&self) -> Vec2;

    /// Return the collision radius of this object
    fn radius(&self) -> f32;

    /// Is this object marked for deletion
    fn is_destroyed(&self) -> bool;
}

impl<T: GameObject> GameObjectArray<T> {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    fn find_potential_collider_slice(&self, left: f32, right: f32) -> (usize, usize) {
        let start = match self
            .0
            .binary_search_by(|obj| (obj.pos().0 + obj.radius()).total_cmp(&left))
        {
            Ok(i) => i,
            Err(i) => i,
        };

        let count = self.0[start..]
            .iter()
            .take_while(|obj| obj.pos().0 - obj.radius() <= right)
            .count();

        (start, start + count)
    }

    pub fn range_slice(&self, left: f32, right: f32) -> &[T] {
        let (start, end) = self.find_potential_collider_slice(left, right);

        &self.0[start..end]
    }

    pub fn range_slice_mut(&mut self, left: f32, right: f32) -> &mut [T] {
        let (start, end) = self.find_potential_collider_slice(left, right);

        &mut self.0[start..end]
    }

    pub fn collider_slice_mut(&mut self, obj: &impl GameObject) -> &mut [T] {
        self.range_slice_mut(obj.pos().0 - obj.radius(), obj.pos().0 + obj.radius())
    }

    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.0.iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, T> {
        self.0.iter_mut()
    }

    pub fn self_collision_iter_mut(&mut self) -> GameObjectTailedIterMut<'_, T> {
        GameObjectTailedIterMut::new(&mut self.0)
    }

    pub fn push(&mut self, obj: T) {
        self.0.push(obj);
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }

    pub fn last_mut(&mut self) -> &mut T {
        self.0.last_mut().unwrap()
    }

    /**
     * Sort the game object array for broadphase collision checks.
     * Also removes destroyed objects.
     */
    pub fn sort(&mut self) {
        self.0.sort_unstable_by(|a, b| {
            let x1 = if a.is_destroyed() {
                f32::MAX
            } else {
                a.pos().0
            };
            let x2 = if b.is_destroyed() {
                f32::MAX
            } else {
                b.pos().0
            };
            x1.total_cmp(&x2)
        });

        let count_destroyed = self.0.iter().rev().take_while(|o| o.is_destroyed()).count();
        if count_destroyed > 0 {
            self.0.truncate(self.0.len() - count_destroyed);
        }
    }
}

pub struct GameObjectTailedIterMut<'a, T> {
    ptr: NonNull<T>,
    remaining: usize,
    _marker: PhantomData<&'a T>,
}

impl<'a, T> GameObjectTailedIterMut<'a, T> {
    fn new(objects: &'a mut [T]) -> Self {
        let len = objects.len();
        let ptr = NonNull::from_ref(objects).cast();
        Self {
            ptr,
            remaining: len,
            _marker: PhantomData,
        }
    }
}

impl<'a, T: GameObject> Iterator for GameObjectTailedIterMut<'a, T> {
    type Item = (&'a mut T, &'a mut [T]);
    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining > 0 {
            let head = unsafe { self.ptr.as_mut() };

            let right_edge = head.pos().0 + head.radius();

            self.ptr = unsafe { self.ptr.add(1) };
            self.remaining -= 1;

            let tail = unsafe { slice::from_raw_parts_mut(self.ptr.as_ptr(), self.remaining) };

            // trim tail to include only objects potentially colliding
            let potentials = tail
                .iter()
                .take_while(|obj| obj.pos().0 - obj.radius() <= right_edge)
                .count();

            let tail = &mut tail[0..potentials];
            Some((head, tail))
        } else {
            None
        }
    }
}
