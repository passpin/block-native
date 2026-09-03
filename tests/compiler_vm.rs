use block_native::{bytecode, model::Project, vm::Runtime};

fn demo_project_json() -> &'static str {
    r#"{
      "version": 1,
      "name": "demo",
      "stage": {"width": 480, "height": 360, "background": [245, 247, 250, 255]},
      "sprites": [{
        "id": "sprite_1",
        "name": "Sprite 1",
        "x": 0.0,
        "y": 0.0,
        "direction": 0.0,
        "size": 28.0,
        "color": [80, 150, 255, 255],
        "script": [
          {"id":"cmd_repeat","op": "repeat", "times": 2, "body": [
            {"id":"cmd_move","op": "move", "steps": 10.0},
            {"id":"cmd_turn","op": "turn", "degrees": 90.0}
          ]},
          {"id":"cmd_wait","op": "wait", "seconds": 0.5}
        ]
      }]
    }"#
}

#[test]
fn version_one_project_upgrades_and_compiles_to_blk2() {
    let project = Project::from_json_str(demo_project_json()).unwrap();
    assert_eq!(project.sprites[0].id, "sprite_1");
    assert_eq!(project.sprites[0].scripts.len(), 1);

    let bytes = bytecode::compile(&project).unwrap();
    assert_eq!(&bytes[..4], b"BLK2");
    let program = bytecode::decode(&bytes).unwrap();
    assert_eq!(program.name, "demo");
    assert_eq!(program.stage.width, 480);
    assert_eq!(program.sprites.len(), 1);
    assert_eq!(program.sprites[0].scripts.len(), 1);
}

#[test]
fn runtime_executes_legacy_motion_turn_and_wait_incrementally() {
    let project = Project::from_json_str(demo_project_json()).unwrap();
    let bytes = bytecode::compile(&project).unwrap();
    let program = bytecode::decode(&bytes).unwrap();
    let mut runtime = Runtime::new(program);

    runtime.update(0.0);
    let sprite = &runtime.sprites()[0];
    assert!((sprite.x - 10.0).abs() < 0.001);
    assert!((sprite.y - 10.0).abs() < 0.001);
    assert!((sprite.direction - 180.0).abs() < 0.001);
    assert!(!runtime.is_finished());

    runtime.update(0.25);
    assert!(!runtime.is_finished());
    runtime.update(0.25);
    assert!(runtime.is_finished());
}
