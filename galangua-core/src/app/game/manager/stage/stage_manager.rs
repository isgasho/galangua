use super::appearance_manager::AppearanceManager;
use super::appearance_manager::Accessor as AccessorForAppearance;
use super::attack_manager::AttackManager;
use super::enemy_manager::EnemyManager;
use super::formation::Formation;

use crate::app::game::enemy::enemy::Enemy;
use crate::app::game::enemy::{Accessor, FormationIndex};
use crate::app::util::collision::CollBox;
use crate::app::util::unsafe_util::peep;
use crate::framework::types::Vec2I;
use crate::framework::RendererTrait;

#[cfg(debug_assertions)]
use crate::app::game::enemy::enemy::create_enemy;

const RUSH_THRESHOLD: u32 = 5;

#[derive(Clone, Copy, PartialEq)]
enum StageState {
    APPEARANCE,
    NORMAL,
    RUSH,
    CLEARED,
}

pub struct StageManager {
    enemy_manager: EnemyManager,
    formation: Formation,
    appearance_manager: AppearanceManager,
    attack_manager: AttackManager,
    stage_state: StageState,
}

impl StageManager {
    pub fn new() -> Self {
        Self {
            enemy_manager: EnemyManager::new(),
            formation: Formation::new(),
            appearance_manager: AppearanceManager::new(0),
            attack_manager: AttackManager::new(),
            stage_state: StageState::APPEARANCE,
        }
    }

    pub fn start_next_stage(&mut self, stage: u16, captured_fighter: Option<FormationIndex>) {
        self.enemy_manager.start_next_stage();
        self.appearance_manager.restart(stage, captured_fighter);
        self.formation.restart();
        self.attack_manager.restart(stage);
        self.stage_state = StageState::APPEARANCE;
    }

    pub fn all_destroyed(&self) -> bool {
        self.stage_state == StageState::CLEARED &&
            self.enemy_manager.all_destroyed()
    }

    pub fn update<T: Accessor>(&mut self, accessor: &mut T) {
        self.update_appearance();
        self.update_formation();
        self.update_attackers(accessor);
        self.enemy_manager.update(accessor);
        self.check_stage_state();
    }

    pub fn draw<R: RendererTrait>(&self, renderer: &mut R) {
        self.enemy_manager.draw(renderer);
    }

    pub fn check_collision<A: Accessor>(
        &mut self, target: &CollBox, power: u32, accessor: &mut A,
    ) -> bool {
        self.enemy_manager.check_collision(target, power, accessor)
    }

    pub fn check_shot_collision(&mut self, target: &CollBox) -> bool {
        self.enemy_manager.check_shot_collision(target)
    }

    fn update_appearance(&mut self) {
        let prev_done = self.appearance_manager.done;
        let accessor = unsafe { peep(self) };
        if let Some(new_borns) = self.appearance_manager.update(accessor) {
            for enemy in new_borns {
                self.enemy_manager.spawn(enemy);
            }
        }
        if !prev_done && self.appearance_manager.done {
            self.stage_state = StageState::NORMAL;
            self.formation.done_appearance();
            self.attack_manager.set_enable(true);
        }
    }

    pub fn spawn_captured_fighter(&mut self, pos: &Vec2I, fi: &FormationIndex) -> bool {
        self.enemy_manager.spawn_captured_fighter(pos, fi)
    }

    pub fn remove_enemy(&mut self, formation_index: &FormationIndex) -> bool {
        self.enemy_manager.remove_enemy(formation_index)
    }

    fn update_formation(&mut self) {
        self.formation.update();
    }

    fn update_attackers<T: Accessor>(&mut self, accessor: &mut T) {
        self.attack_manager.update(accessor);
    }

    fn check_stage_state(&mut self) {
        if self.stage_state == StageState::APPEARANCE {
            return;
        }

        let new_state = match self.enemy_manager.alive_enemy_count {
            n if n == 0               => StageState::CLEARED,
            n if n <= RUSH_THRESHOLD  => StageState::RUSH,
            _                         => self.stage_state,
        };
        if new_state != self.stage_state {
            self.stage_state = new_state;
        }
    }

    pub fn pause_enemy_shot(&mut self, wait: u32) {
        self.enemy_manager.pause_enemy_shot(wait);
    }

    pub fn spawn_shot(&mut self, pos: &Vec2I, target_pos: &[Option<Vec2I>], speed: i32) {
        self.enemy_manager.spawn_shot(pos, target_pos, speed);
    }

    pub fn pause_attack(&mut self, value: bool) {
        self.attack_manager.pause(value);
        self.appearance_manager.pause(value);
    }

    pub fn is_no_attacker(&self) -> bool {
        self.attack_manager.is_no_attacker()
    }

    pub fn get_enemy_at(&self, formation_index: &FormationIndex) -> Option<&Box<dyn Enemy>> {
        self.enemy_manager.get_enemy_at(formation_index)
    }

    pub fn get_enemy_at_mut(&mut self, formation_index: &FormationIndex) -> Option<&mut Box<dyn Enemy>> {
        self.enemy_manager.get_enemy_at_mut(formation_index)
    }

    pub fn get_formation_pos(&self, formation_index: &FormationIndex) -> Vec2I {
        self.formation.pos(formation_index)
    }

    pub fn is_rush(&self) -> bool {
        self.stage_state == StageState::RUSH
    }

    // Debug

    #[cfg(debug_assertions)]
    pub fn reset_stable(&mut self) {
        self.enemy_manager.reset_stable();

        let stage = 0;
        self.appearance_manager.restart(stage, None);
        self.appearance_manager.done = true;
        self.formation.restart();
        self.formation.done_appearance();
        self.attack_manager.restart(stage);
        self.attack_manager.set_enable(false);
        self.stage_state = StageState::NORMAL;

        for unit in 0..5 {
            for i in 0..8 {
                let index = super::appearance_table::ORDER[unit * 8 + i];
                let enemy_type = super::appearance_table::ENEMY_TYPE_TABLE[unit * 2 + (i / 4)];
                let pos = self.formation.pos(&index);
                let mut enemy = create_enemy(enemy_type, &pos, 0, 0, &index);
                enemy.set_to_formation();
                self.enemy_manager.spawn(enemy);
            }
        }
    }
}

impl AccessorForAppearance for StageManager {
    fn is_stationary(&self) -> bool {
        self.enemy_manager.is_stationary()
    }
}
