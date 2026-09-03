use block_native::{bytecode, model::Value, parser::parse_project, vm::Runtime};

fn runtime_source() -> &'static str {
    r#"
project "runtime" {
  stage 480 360 background #ffffffff
  global score = 0
  list log = []
  sprite "Cat" at 0 0 direction 0 size 20 color #4c97ffff {
    var speed = 2
    var loops = 0
    list trail = []
    when start {
      repeat 3 {
        move speed
        change score by 1
        push score to trail
      }
      while loops < 2 {
        change loops by 1
        move 1
      }
      if score == 3 {
        call hop(4)
      } else {
        move 100
      }
      broadcast "go"
      wait 0.5
    }
    when message "go" {
      turn 90
    }
    when key "space" {
      change score by 10
    }
    proc hop(amount) {
      move amount
    }
  }
}
"#
}

#[test]
fn compiler_emits_blk2_and_round_trips_program() {
    let project = parse_project(runtime_source()).unwrap();
    let bytes = bytecode::compile(&project).unwrap();
    assert_eq!(&bytes[..4], b"BLK2");
    let program = bytecode::decode(&bytes).unwrap();
    assert_eq!(program.name, "runtime");
    assert_eq!(program.sprites.len(), 1);
    assert_eq!(program.sprites[0].scripts.len(), 3);
    assert_eq!(program.sprites[0].procedures.len(), 1);
}

#[test]
fn scheduler_executes_state_control_procedure_broadcast_wait_and_key_event() {
    let project = parse_project(runtime_source()).unwrap();
    let program = bytecode::decode(&bytecode::compile(&project).unwrap()).unwrap();
    let mut runtime = Runtime::new(program);

    runtime.update(0.0);
    runtime.update(0.0); // processes the message thread spawned by broadcast

    let sprite = &runtime.sprites()[0];
    assert!((sprite.x - 12.0).abs() < 0.001);
    assert!((sprite.direction - 90.0).abs() < 0.001);
    assert_eq!(runtime.global_value("score"), Some(&Value::Number(3.0)));
    assert_eq!(runtime.sprite_value(0, "loops"), Some(&Value::Number(2.0)));
    assert_eq!(runtime.sprite_list_len(0, "trail"), Some(3));
    assert!(!runtime.is_finished());

    runtime.set_key("space", true);
    runtime.update(0.0);
    assert_eq!(runtime.global_value("score"), Some(&Value::Number(13.0)));
    runtime.set_key("space", false);

    runtime.update(0.5);
    assert!(runtime.is_finished());
}

#[test]
fn pen_records_motion_and_touching_expression_is_available() {
    let source = r#"
project "pen" {
  stage 480 360 background #ffffffff
  sprite "A" at 0 0 direction 0 size 20 color #ff0000ff {
    when start {
      pen down
      move 10
      pen up
      if touching("B") { turn 45 }
    }
  }
  sprite "B" at 15 0 direction 0 size 20 color #00ff00ff {
    when start { wait 1 }
  }
}
"#;
    let project = parse_project(source).unwrap();
    let program = bytecode::decode(&bytecode::compile(&project).unwrap()).unwrap();
    let mut runtime = Runtime::new(program);
    runtime.update(0.0);

    assert_eq!(runtime.pen_segments().len(), 1);
    assert!((runtime.sprites()[0].direction - 45.0).abs() < 0.001);
}
