use bb_fme_bevy::domain::CardinalDirection;

#[test]
fn unity_direction_indices_are_preserved() {
    let expected = [
        CardinalDirection::Up,
        CardinalDirection::Right,
        CardinalDirection::Down,
        CardinalDirection::Left,
    ];

    for (index, expected_direction) in expected.into_iter().enumerate() {
        let direction = CardinalDirection::try_from(index as i32).expect("0..=3 must be valid");

        assert_eq!(direction, expected_direction);
        assert_eq!(direction.index(), index as i32);
    }
}

#[test]
fn direction_offsets_match_unity_coordinates() {
    assert_eq!(CardinalDirection::Up.unit_offset(), (0, 1));

    assert_eq!(CardinalDirection::Right.unit_offset(), (1, 0));

    assert_eq!(CardinalDirection::Down.unit_offset(), (0, -1));

    assert_eq!(CardinalDirection::Left.unit_offset(), (-1, 0));
}

#[test]
fn four_clockwise_turns_return_to_start() {
    let mut direction = CardinalDirection::Up;

    for _ in 0..4 {
        direction = direction.clockwise();
    }

    assert_eq!(direction, CardinalDirection::Up);
}

#[test]
fn unity_rotation_angles_are_clockwise() {
    assert_eq!(CardinalDirection::Up.unity_angle_degrees(), 0);

    assert_eq!(CardinalDirection::Right.unity_angle_degrees(), -90);

    assert_eq!(CardinalDirection::Down.unity_angle_degrees(), -180);

    assert_eq!(CardinalDirection::Left.unity_angle_degrees(), -270);
}

#[test]
fn invalid_direction_is_rejected() {
    let error = CardinalDirection::try_from(4).expect_err("4 must not be a cardinal direction");

    assert_eq!(error.value(), 4);
    assert_eq!(
        error.to_string(),
        "direction index must be in 0..=3, found 4"
    );
}
