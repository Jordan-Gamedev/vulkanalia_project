use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::rc::{Rc, Weak};
use std::cell::RefCell;

use crate::engine::App;

pub trait Component: Clone {
    fn on_add(&mut self, world: &mut World);
    fn on_remove(&self, world: &mut World);
}

trait ComponentStorage: Any {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn push_entity_slot(&mut self);
    fn swap_remove_entity(&mut self, entity_index: usize);
    fn has_component(&self, entity: u32) -> bool;
    fn remove_component_for_entity(&mut self, entity_index: usize);
}

// fn main() {
//     #[derive(Debug, PartialEq)]
//     struct Position(f32, f32);

//     #[derive(Debug, PartialEq)]
//     struct Velocity(f32, f32);

//     #[derive(Debug, PartialEq)]
//     struct Health(u32);

//     let mut world = World::new();

//     let e0 = world.create_entity();
//     let e1 = world.create_entity();
//     let e2 = world.create_entity();
//     let e3 = world.create_entity();

//     world.add_component(e0, Position(1.0, 1.5));
//     world.add_component(e1, Position(2.0, 2.5));
//     world.add_component(e1, Velocity(0.5, 0.25));
//     world.add_component(e2, Velocity(1.5, 1.25));
//     world.add_component(e3, Position(4.0, 4.5));
//     world.add_component(e3, Velocity(2.5, 2.25));

//     assert!(world.query::<Health>().is_none(), "query for a missing component type should return None");

//     {
//         let (positions, entity_ids) = world.query::<Position>().expect("query::<Position> should return matches");
//         assert_eq!(entity_ids, vec![e0, e1, e3]);
//         assert_eq!(positions.len(), 3);
//         assert_eq!((positions[0].0, positions[0].1), (1.0, 1.5));
//         assert_eq!((positions[1].0, positions[1].1), (2.0, 2.5));
//         assert_eq!((positions[2].0, positions[2].1), (4.0, 4.5));

//         positions[0].0 += 10.0;
//         positions[2].1 += 20.0;
//     }

//     assert_eq!(world.get_component::<Position>(e0), Some(&Position(11.0, 1.5)));
//     assert_eq!(world.get_component::<Position>(e1), Some(&Position(2.0, 2.5)));
//     assert_eq!(world.get_component::<Position>(e3), Some(&Position(4.0, 24.5)));

//     let queried_entities: Vec<u32>;
//     {
//         let (mut positions, mut velocities, entity_ids) = world.query2::<Position, Velocity>().expect("query2 should find shared entities");
//         assert_eq!(entity_ids, vec![e1, e3]);
//         assert_eq!(positions.len(), 2);
//         assert_eq!(velocities.len(), 2);

//         positions[0].1 += 1.0;
//         velocities[1].0 += 5.0;

//         queried_entities = entity_ids;
//     }

//     let first_queried = queried_entities[0];
//     let second_queried = queried_entities[1];

//     if let Some(position) = world.get_component_mut::<Position>(first_queried) {
//         position.0 += 2.0;
//         position.1 += 2.0;
//     } else {
//         panic!("expected queried entity to still have Position");
//     }

//     if let Some(velocity) = world.get_component_mut::<Velocity>(second_queried) {
//         velocity.0 += 3.0;
//         velocity.1 += 3.0;
//     } else {
//         panic!("expected queried entity to still have Velocity");
//     }

//     assert_eq!(world.get_component::<Position>(e1), Some(&Position(4.0, 5.5)));
//     assert_eq!(world.get_component::<Velocity>(e3), Some(&Velocity(10.5, 5.25)));

//     println!("ecs query tests completed successfully.");
// }

struct ComponentTypeStorage<T> {
    component_data: Vec<T>,
    component_indices_by_entity: Vec<u32>,
}

impl<T> ComponentTypeStorage<T> {
    fn new(entity_count: u32) -> Self {
        Self {
            component_data: Vec::new(),
            component_indices_by_entity: vec![u32::MAX; entity_count as usize],
        }
    }
}

impl<T: 'static> ComponentTypeStorage<T> {
    fn has_component(&self, entity: u32) -> bool {
        self.component_indices_by_entity[entity as usize] != u32::MAX
    }

    fn get_component(&self, entity: u32) -> Option<&T> {
        let comp_index = self.component_indices_by_entity.get(entity as usize).copied().unwrap_or(u32::MAX);
        if comp_index == u32::MAX {
            None
        } else {
            self.component_data.get(comp_index as usize)
        }
    }

    fn get_component_mut(&mut self, entity: u32) -> Option<&mut T> {
        let comp_index = self.component_indices_by_entity.get(entity as usize).copied().unwrap_or(u32::MAX);
        if comp_index == u32::MAX {
            None
        } else {
            self.component_data.get_mut(comp_index as usize)
        }
    }

    fn add_component(&mut self, entity: u32, component: T) {
        self.component_data.push(component);
        self.component_indices_by_entity[entity as usize] = (self.component_data.len() - 1) as u32;
    }

    fn remove_component(&mut self, entity_index: usize) {
        if self.component_indices_by_entity[entity_index] != u32::MAX {
            let comp_index = self.component_indices_by_entity[entity_index] as usize;
            if !self.component_data.is_empty() {
                let last_index = self.component_data.len() - 1;
                if let Some(pos) = self.component_indices_by_entity.iter_mut().rev().find(|f| **f == last_index as u32) {
                    *pos = comp_index as u32;
                }
                self.component_data.swap_remove(comp_index);
                self.component_indices_by_entity[entity_index] = u32::MAX;
            }
        }
    }
}

impl<T: 'static> ComponentStorage for ComponentTypeStorage<T> {
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }

    fn push_entity_slot(&mut self) {
        self.component_indices_by_entity.push(u32::MAX);
    }

    fn swap_remove_entity(&mut self, entity_index: usize) {
        self.component_indices_by_entity.swap_remove(entity_index);
    }

    fn has_component(&self, entity: u32) -> bool {
        self.has_component(entity)
    }

    fn remove_component_for_entity(&mut self, entity_index: usize) {
        self.remove_component(entity_index);
    }
}

#[derive(Default)]
pub struct World {
    pub app: *mut App,
    component_type_storages: HashMap<TypeId, Box<dyn ComponentStorage>>,
    active_entity_references: Vec<Weak<RefCell<EntityRef>>>,
    entity_count: u32,
}

impl World {
    pub fn new(app: *mut App) -> Self {
        Self {
            app,
            component_type_storages: HashMap::new(),
            active_entity_references: Vec::new(),
            entity_count: 0,
        }
    }

    pub fn create_entity(&mut self) -> u32 {
        for storage in self.component_type_storages.values_mut() {
            storage.push_entity_slot();
        }
        self.entity_count += 1;
        self.entity_count - 1
    }

    pub fn destroy_entity(&mut self, entity: u32) {
        if entity >= self.entity_count {
            return
        }

        // Update any tracked EntityRef entries that referenced the removed entity.
        // We can't mutably change the contents of an `Rc` if other clones exist,
        // so replace the Rc in the vector with a fresh one containing the new value.
        // First, prune dead weak refs and simultaneously update live ones.
        self.active_entity_references.retain(|weak| {
            if let Some(rc) = weak.upgrade() {
                let mut r = rc.borrow_mut();
                if r.entity == entity {
                    r.entity = u32::MAX;
                }
                true
            } else {
                // weak couldn't be upgraded -> target dropped, remove this slot
                false
            }
        });

        // If any references pointed to the last entity (which will move into `entity`'s slot),
        // update those references to point to `entity` instead.
        let last = self.entity_count - 1;
        for weak in self.active_entity_references.iter() {
            if let Some(rc) = weak.upgrade() {
                let mut r = rc.borrow_mut();
                if r.entity == last {
                    r.entity = entity;
                }
            }
        }

        self.remove_all_components(entity);
        for storage in self.component_type_storages.values_mut() {
            storage.swap_remove_entity(entity as usize);
        }
        self.entity_count -= 1;
    }

    pub fn has_component<T: 'static>(&self, entity: u32) -> bool {
        if entity >= self.entity_count {
            return false
        }
        
        if let Some(boxed) = self.component_type_storages.get(&TypeId::of::<T>()) {
            if let Some(concrete) = boxed.as_any().downcast_ref::<ComponentTypeStorage<T>>() {
                return concrete.has_component(entity)
            }
        }
        false
    }

    pub fn get_component<T: 'static>(&self, entity: u32) -> Option<&T> {
        if entity >= self.entity_count {
            return None
        }
        
        if let Some(boxed_storage) = self.component_type_storages.get(&TypeId::of::<T>()) {
            let storage = boxed_storage.as_any().downcast_ref::<ComponentTypeStorage<T>>()?;
            storage.get_component(entity)
        } else { None }
    }

    pub fn get_component_mut<T: 'static>(&mut self, entity: u32) -> Option<&mut T> {
        if entity >= self.entity_count {
            return None
        }
        
        if let Some(boxed_storage) = self.component_type_storages.get_mut(&TypeId::of::<T>()) {
            let storage = boxed_storage.as_any_mut().downcast_mut::<ComponentTypeStorage<T>>()?;
            storage.get_component_mut(entity)
        } else { None }
    }

    pub fn query<T: 'static>(&mut self) -> Vec<(&mut T, u32)> {
        match self.component_type_storages.get_mut(&TypeId::of::<T>()) {
            Some(boxed_storage) => {
                let storage = boxed_storage.as_any_mut().downcast_mut::<ComponentTypeStorage<T>>().unwrap();
                let components: Vec<&mut T> = storage.component_data.iter_mut().collect();
                let entities: Vec<u32> = storage.component_indices_by_entity
                    .iter()
                    .enumerate()
                    .filter(|(_, comp_index)| **comp_index != u32::MAX)
                    .map(|(i, _)| i as u32)
                    .collect();

                components.into_iter().zip(entities).collect()
            },
            None => {
                let query: Vec<(&mut T, u32)> = Vec::new();
                query
            },
        }
    }

    pub fn query2<T: 'static, U: 'static>(&mut self) -> Option<(Vec<&mut T>, Vec<&mut U>, Vec<u32>)> {
        let t_id = TypeId::of::<T>();
        let u_id = TypeId::of::<U>();

        if t_id == u_id {
            return None;
        }
    
        if !self.component_type_storages.contains_key(&t_id)
        || !self.component_type_storages.contains_key(&u_id) {
            return None
        }

        // First pass: collect entities with component T
        let shared_entities: Vec<u32> = {
            let boxed_storage_t = self.component_type_storages.get_mut(&t_id)?;
            let storage_t = boxed_storage_t.as_any_mut().downcast_mut::<ComponentTypeStorage<T>>()?;
            storage_t.component_indices_by_entity
                .iter()
                .enumerate()
                .filter(|(_, comp_index)| **comp_index != u32::MAX)
                .map(|(i, _)| i as u32)
                .collect()
        };
        
        // Second pass: filter by component U
        let shared_entities: Vec<u32> = {
            let boxed_storage_u = self.component_type_storages.get_mut(&u_id)?;
            let storage_u = boxed_storage_u.as_any_mut().downcast_mut::<ComponentTypeStorage<U>>()?;
            shared_entities
                .iter()
                .filter(|&&f| storage_u.component_indices_by_entity[f as usize] != u32::MAX)
                .map(|m| *m)
                .collect()
        };
        
        if shared_entities.is_empty() {
            return None;
        }

        let storage_t_ptr = {
            let boxed_storage_t = self.component_type_storages.get_mut(&t_id)?;
            let storage_t = boxed_storage_t.as_any_mut().downcast_mut::<ComponentTypeStorage<T>>()?;
            storage_t as *mut ComponentTypeStorage<T>
        };
        
        let storage_u_ptr = {
            let boxed_storage_u = self.component_type_storages.get_mut(&u_id)?;
            let storage_u = boxed_storage_u.as_any_mut().downcast_mut::<ComponentTypeStorage<U>>()?;
            storage_u as *mut ComponentTypeStorage<U>
        };
        
        unsafe {
            let component_indices_by_entity_t = &(*storage_t_ptr).component_indices_by_entity;
            let component_indices_by_entity_u = &(*storage_u_ptr).component_indices_by_entity;

            let components_t = shared_entities
                .iter()
                .map(|&entity| {
                    let component_index = component_indices_by_entity_t[entity as usize] as usize;
                    &mut *(*storage_t_ptr).component_data.as_mut_ptr().add(component_index)
                })
                .collect();

            let components_u = shared_entities
                .iter()
                .map(|&entity| {
                    let component_index = component_indices_by_entity_u[entity as usize] as usize;
                    &mut *(*storage_u_ptr).component_data.as_mut_ptr().add(component_index)
                })
                .collect();

            Some((components_t, components_u, shared_entities))
        }
        
    }

    pub fn query3<T: 'static, U: 'static, V: 'static>(&mut self) -> Option<(Vec<&mut T>, Vec<&mut U>, Vec<&mut V>, Vec<u32>)> {
        let t_id = TypeId::of::<T>();
        let u_id = TypeId::of::<U>();
        let v_id = TypeId::of::<V>();

        if t_id == u_id || t_id == v_id || u_id == v_id {
            return None;
        }
    
        if !self.component_type_storages.contains_key(&t_id)
        || !self.component_type_storages.contains_key(&u_id)
        || !self.component_type_storages.contains_key(&v_id) {
            return None
        }

        // First pass: collect entities with component T
        let shared_entities: Vec<u32> = {
            let boxed_storage_t = self.component_type_storages.get_mut(&t_id)?;
            let storage_t = boxed_storage_t.as_any_mut().downcast_mut::<ComponentTypeStorage<T>>()?;
            storage_t.component_indices_by_entity
                .iter()
                .enumerate()
                .filter(|(_, comp_index)| **comp_index != u32::MAX)
                .map(|(i, _)| i as u32)
                .collect()
        };
        
        // Second pass: filter by component U
        let shared_entities: Vec<u32> = {
            let boxed_storage_u = self.component_type_storages.get_mut(&u_id)?;
            let storage_u = boxed_storage_u.as_any_mut().downcast_mut::<ComponentTypeStorage<U>>()?;
            shared_entities
                .iter()
                .filter(|&&f| storage_u.component_indices_by_entity[f as usize] != u32::MAX)
                .map(|m| *m)
                .collect()
        };
        
        if shared_entities.is_empty() {
            return None;
        }

        // Third pass: filter by component V
        let shared_entities: Vec<u32> = {
            let boxed_storage_v = self.component_type_storages.get_mut(&v_id)?;
            let storage_v = boxed_storage_v.as_any_mut().downcast_mut::<ComponentTypeStorage<V>>()?;
            shared_entities
                .iter()
                .filter(|&&f| storage_v.component_indices_by_entity[f as usize] != u32::MAX)
                .map(|m| *m)
                .collect()
        };

        if shared_entities.is_empty() {
            return None;
        }

        let storage_t_ptr = {
            let boxed_storage_t = self.component_type_storages.get_mut(&t_id)?;
            let storage_t = boxed_storage_t.as_any_mut().downcast_mut::<ComponentTypeStorage<T>>()?;
            storage_t as *mut ComponentTypeStorage<T>
        };
        
        let storage_u_ptr = {
            let boxed_storage_u = self.component_type_storages.get_mut(&u_id)?;
            let storage_u = boxed_storage_u.as_any_mut().downcast_mut::<ComponentTypeStorage<U>>()?;
            storage_u as *mut ComponentTypeStorage<U>
        };
        
        let storage_v_ptr = {
            let boxed_storage_v = self.component_type_storages.get_mut(&v_id)?;
            let storage_v = boxed_storage_v.as_any_mut().downcast_mut::<ComponentTypeStorage<V>>()?;
            storage_v as *mut ComponentTypeStorage<V>
        };

        unsafe {
            let component_indices_by_entity_t = &(*storage_t_ptr).component_indices_by_entity;
            let component_indices_by_entity_u = &(*storage_u_ptr).component_indices_by_entity;
            let component_indices_by_entity_v = &(*storage_v_ptr).component_indices_by_entity;

            let components_t = shared_entities
                .iter()
                .map(|&entity| {
                    let component_index = component_indices_by_entity_t[entity as usize] as usize;
                    &mut *(*storage_t_ptr).component_data.as_mut_ptr().add(component_index)
                })
                .collect();

            let components_u = shared_entities
                .iter()
                .map(|&entity| {
                    let component_index = component_indices_by_entity_u[entity as usize] as usize;
                    &mut *(*storage_u_ptr).component_data.as_mut_ptr().add(component_index)
                })
                .collect();

            let components_v = shared_entities
                .iter()
                .map(|&entity| {
                    let component_index = component_indices_by_entity_v[entity as usize] as usize;
                    &mut *(*storage_v_ptr).component_data.as_mut_ptr().add(component_index)
                })
                .collect();

            Some((components_t, components_u, components_v, shared_entities))
        }
        
    }

    pub fn get_all_components(&self, entity: u32) -> Vec<&TypeId> {
        if entity >= self.entity_count {
            return Vec::new()
        }
        
        self.component_type_storages
            .iter()
            .filter(|(_k, v)| v.has_component(entity))
            .map(|(k, _v)| k)
            .collect::<Vec<&TypeId>>()
    }

    pub fn add_component<T: 'static>(&mut self, entity: u32, component: T) where T: Component {
        if entity >= self.entity_count {
            return
        }
        
        // Add component type if it does not exist
        if !self.component_type_storages.contains_key(&TypeId::of::<T>()) {
            self.component_type_storages.insert(TypeId::of::<T>(), Box::new(ComponentTypeStorage::<T>::new(self.entity_count)));
        }

        // Call on_add method implementation for this component
        let mut component = component.clone();
        component.on_add(self);

        // Add component to the entity (downcast to concrete storage)
        let boxed = self.component_type_storages.get_mut(&TypeId::of::<T>()).unwrap();
        let concrete = boxed.as_any_mut().downcast_mut::<ComponentTypeStorage<T>>().unwrap();
        concrete.add_component(entity, component);
    }

    pub fn remove_component<T: 'static>(&mut self, entity: u32) where T: Component {
        if entity >= self.entity_count {
            return
        }
        
        // Call on_remove method implementation for this component
        if let Some(component) = self.get_component::<T>(entity) {
            component.clone().on_remove(self);
        }

        if let Some(boxed) = self.component_type_storages.get_mut(&TypeId::of::<T>()) {
            if let Some(concrete) = boxed.as_any_mut().downcast_mut::<ComponentTypeStorage<T>>() {
                concrete.remove_component(entity as usize);
            }
        }
    }

    pub fn remove_all_components(&mut self, entity: u32) {
        if entity >= self.entity_count {
            return
        }
        
        for storage in self.component_type_storages.values_mut() {
            storage.remove_component_for_entity(entity as usize);
        }
    }
}

pub struct EntityRef {
    entity: u32,
}

impl EntityRef {
    pub fn new(entity: u32, world: &mut World) -> Rc<RefCell<Self>> {
        let entity_ref = Self { entity };
        let rc = Rc::new(RefCell::new(entity_ref));
        world.active_entity_references.push(Rc::downgrade(&rc));
        rc
    }

    pub fn is_valid(&self) -> bool {
        self.entity != u32::MAX
    }
}