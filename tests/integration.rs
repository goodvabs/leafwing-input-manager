#![cfg(test)]
use bevy::ecs::query::ChangeTrackers;
use bevy::prelude::*;
use leafwing_input_manager::prelude::*;
use leafwing_input_manager::MockInput;

#[derive(Actionlike, Clone, Copy, Debug)]
enum Action {
    PayRespects,
}

// A resource that represents whether respects have been paid or not
#[derive(Default, PartialEq, Debug)]
struct Respect(bool);

fn pay_respects(
    action_state: Query<&ActionState<Action>, With<Player>>,
    mut respect: ResMut<Respect>,
) {
    if action_state.single().pressed(Action::PayRespects) {
        respect.0 = true;
    }
}

fn respect_fades(mut respect: ResMut<Respect>) {
    respect.0 = false;
}

#[derive(Component)]
struct Player;

fn spawn_player(mut commands: Commands) {
    commands
        .spawn()
        .insert(Player)
        .insert_bundle(InputManagerBundle::<Action> {
            input_map: InputMap::<Action>::new([(Action::PayRespects, KeyCode::F)]),
            ..Default::default()
        });
}

// Normally this is done by the winit_plugin
fn reset_inputs(world: &mut World) {
    world.reset_inputs();
}

#[test]
fn action_state_change_detection() {
    use bevy::input::InputPlugin;

    let mut app = App::new();

    app.add_plugins(MinimalPlugins)
        .add_plugin(InputPlugin)
        .add_plugin(InputManagerPlugin::<Action>::default())
        .add_startup_system(spawn_player)
        .add_system(action_state_changed_iff_input_changed);

    for i in 0..10 {
        if i % 2 == 0 {
            app.send_input(KeyCode::F);
        }

        app.update();
    }

    fn action_state_changed_iff_input_changed(
        query: Query<ChangeTrackers<ActionState<Action>>>,
        input: Res<Input<KeyCode>>,
    ) {
        let action_state_tracker = query.single();

        if input.is_changed() {
            assert!(action_state_tracker.is_changed());
        } else {
            assert!(!action_state_tracker.is_changed());
        }
    }
}

#[test]
fn run_in_state() {
    use bevy::input::InputPlugin;

    #[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
    enum GameState {
        Playing,
        Paused,
    }

    let mut app = App::new();

    app.add_plugins(MinimalPlugins)
        .add_plugin(InputPlugin)
        .add_system_to_stage(CoreStage::Last, reset_inputs.exclusive_system())
        .add_plugin(InputManagerPlugin::<Action, GameState>::run_in_state(
            GameState::Playing,
        ))
        .add_startup_system(spawn_player)
        .add_state(GameState::Playing)
        .init_resource::<Respect>()
        .add_system(pay_respects)
        .add_system_to_stage(CoreStage::PreUpdate, respect_fades);

    // Press F to pay respects
    app.send_input(KeyCode::F);
    app.update();
    let respect = app.world.get_resource::<Respect>().unwrap();
    assert_eq!(*respect, Respect(true));

    // Pause the game
    let mut game_state = app.world.get_resource_mut::<State<GameState>>().unwrap();
    game_state.set(GameState::Paused).unwrap();

    // Now, all respect has faded
    app.update();
    let respect = app.world.get_resource::<Respect>().unwrap();
    assert_eq!(*respect, Respect(false));

    // And even pressing F cannot bring it back
    app.send_input(KeyCode::F);
    app.update();
    let respect = app.world.get_resource::<Respect>().unwrap();
    assert_eq!(*respect, Respect(false));
}

#[test]
#[cfg(feature = "ui")]
fn action_state_driver() {
    use bevy::input::InputPlugin;

    let mut app = App::new();

    #[derive(Component)]
    struct ButtonMarker;

    fn setup(mut commands: Commands) {
        let player_entity = commands
            .spawn()
            .insert(Player)
            .insert_bundle(InputManagerBundle::<Action> {
                input_map: InputMap::<Action>::new([(Action::PayRespects, KeyCode::F)]),
                ..Default::default()
            })
            .id();

        commands
            .spawn()
            .insert(ButtonMarker)
            .insert(Interaction::None)
            .insert(ActionStateDriver::<Action> {
                action: Action::PayRespects,
                entity: player_entity,
            });
    }

    app.add_plugins(MinimalPlugins)
        .add_plugin(InputManagerPlugin::<Action>::default())
        .add_plugin(InputPlugin)
        .add_startup_system(setup)
        .add_system(pay_respects)
        .add_system_to_stage(CoreStage::PreUpdate, respect_fades)
        .init_resource::<Respect>();

    app.update();

    let respect = app.world.get_resource::<Respect>().unwrap();
    assert_eq!(*respect, Respect(false));

    // Click button to pay respects
    app.click_button::<ButtonMarker>();

    // Verify that the button was in fact clicked
    let mut button_query = app.world.query::<&Interaction>();
    let interaction = button_query.iter(&app.world).next().unwrap();
    assert_eq!(*interaction, Interaction::Clicked);

    // Run the app once to process the clicks
    app.update();

    // Check the action state
    let mut action_state_query = app.world.query::<&ActionState<Action>>();
    let action_state = action_state_query.iter(&app.world).next().unwrap();
    assert!(action_state.pressed(Action::PayRespects));

    // Check the effects of that action state
    let respect = app.world.get_resource::<Respect>().unwrap();
    assert_eq!(*respect, Respect(true));

    // Clear inputs
    app.reset_inputs();
    app.update();

    let respect = app.world.get_resource::<Respect>().unwrap();
    assert_eq!(*respect, Respect(false));
}
