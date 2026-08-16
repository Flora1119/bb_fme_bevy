use super::{
    ActivePlayWorld, BLOCK_WORLD_SIZE, MIN_VIEW_HEIGHT, MIN_VIEW_WIDTH, MapCamera, PlayWorld,
    PlayerBall,
};
use bevy::{camera::CameraUpdateSystems, prelude::*, transform::TransformSystems};

pub const CAMERA_VERTICAL_DEAD_ZONE: f32 = 3.0;

pub struct PlayerCameraPlugin;

impl Plugin for PlayerCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            follow_player_camera
                .after(CameraUpdateSystems)
                .before(TransformSystems::Propagate),
        );
    }
}

fn follow_player_camera(
    active_play_world: Res<ActivePlayWorld>,
    play_worlds: Query<&PlayWorld>,
    players: Query<&Transform, (With<PlayerBall>, Without<MapCamera>)>,
    mut cameras: Query<(&mut Transform, &Projection), (With<MapCamera>, Without<PlayerBall>)>,
) {
    let Some(root) = active_play_world.root() else {
        return;
    };

    let Ok(play_world) = play_worlds.get(root) else {
        return;
    };

    let Ok(player_transform) = players.single() else {
        return;
    };

    let map_size = play_world.definition().settings.size;

    for (mut camera_transform, projection) in &mut cameras {
        let Projection::Orthographic(orthographic) = projection else {
            continue;
        };

        let visible_size = orthographic_view_size(orthographic);

        let desired_y = vertical_dead_zone_target(
            player_transform.translation.y,
            camera_transform.translation.y,
            CAMERA_VERTICAL_DEAD_ZONE,
        );

        camera_transform.translation.x = clamp_camera_axis(
            player_transform.translation.x,
            map_size.width,
            visible_size.x,
        );

        camera_transform.translation.y =
            clamp_camera_axis(desired_y, map_size.height, visible_size.y);
    }
}

fn orthographic_view_size(orthographic: &OrthographicProjection) -> Vec2 {
    let actual = orthographic.area.size();

    Vec2::new(
        sanitize_view_axis(actual.x, MIN_VIEW_WIDTH),
        sanitize_view_axis(actual.y, MIN_VIEW_HEIGHT),
    )
}

fn sanitize_view_axis(actual: f32, minimum: f32) -> f32 {
    if actual.is_finite() {
        actual.max(minimum)
    } else {
        minimum
    }
}

fn clamp_camera_axis(target: f32, map_cell_count: i32, visible_world_size: f32) -> f32 {
    let map_span = (map_cell_count - 1).max(0) as f32 * BLOCK_WORLD_SIZE;

    // 화면 가장자리에는 블록 중심이 아니라 블록 바깥 면이 닿으므로,
    // 표시 크기에서 블록 한 칸을 제외한 중심 간격을 사용합니다.
    let visible_center_span = (visible_world_size - BLOCK_WORLD_SIZE).max(0.0);

    if map_span <= visible_center_span {
        return map_span * 0.5;
    }

    let half_visible_span = visible_center_span * 0.5;
    let minimum = half_visible_span;
    let maximum = map_span - half_visible_span;

    target.clamp(minimum, maximum)
}

fn vertical_dead_zone_target(player_y: f32, camera_y: f32, dead_zone: f32) -> f32 {
    let dead_zone = dead_zone.max(0.0);
    let offset = player_y - camera_y;

    if offset > dead_zone {
        player_y - dead_zone
    } else if offset < -dead_zone {
        player_y + dead_zone
    } else {
        camera_y
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 0.000_001;

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() <= EPSILON,
            "expected {expected}, found {actual}"
        );
    }

    #[test]
    fn camera_is_centered_when_map_fits_inside_view() {
        assert_close(clamp_camera_axis(-100.0, 25, 25.0), 12.0);
        assert_close(clamp_camera_axis(100.0, 25, 25.0), 12.0);

        assert_close(clamp_camera_axis(-100.0, 15, 15.0), 7.0);
        assert_close(clamp_camera_axis(100.0, 15, 15.0), 7.0);
    }

    #[test]
    fn camera_is_clamped_at_both_edges_of_a_large_map() {
        assert_close(clamp_camera_axis(0.0, 45, 25.0), 12.0);
        assert_close(clamp_camera_axis(22.0, 45, 25.0), 22.0);
        assert_close(clamp_camera_axis(44.0, 45, 25.0), 32.0);

        assert_close(clamp_camera_axis(0.0, 30, 15.0), 7.0);
        assert_close(clamp_camera_axis(29.0, 30, 15.0), 22.0);
    }

    #[test]
    fn vertical_camera_moves_only_outside_dead_zone() {
        assert_close(vertical_dead_zone_target(9.5, 7.0, 3.0), 7.0);
        assert_close(vertical_dead_zone_target(4.5, 7.0, 3.0), 7.0);

        assert_close(vertical_dead_zone_target(10.5, 7.0, 3.0), 7.5);
        assert_close(vertical_dead_zone_target(2.5, 7.0, 3.0), 5.5);
    }
}
