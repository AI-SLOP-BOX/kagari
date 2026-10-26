#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Axis2 {
    X,
    Y,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Axis3 {
    X,
    Y,
    Z,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Vector2Track {
    Position,
    Scale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Vector3Track {
    Position,
    Rotation,
    Scale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GraphProperty<'a> {
    Vector2 {
        track: Vector2Track,
        axis: Axis2,
    },
    Scalar2d(Scalar2dTrack),
    Vector3 {
        track: Vector3Track,
        axis: Axis3,
    },
    PuppetPin {
        id: &'a str,
        axis: Axis2,
    },
    Effect {
        effect_id: &'a str,
        parameter: &'a str,
        component: Option<usize>,
    },
    Unknown(&'a str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Scalar2dTrack {
    Rotation,
    Opacity,
}

impl<'a> GraphProperty<'a> {
    pub(crate) fn parse(key: &'a str) -> Self {
        let vector2 = match key {
            "Position X" => Some((Vector2Track::Position, Axis2::X)),
            "Position Y" => Some((Vector2Track::Position, Axis2::Y)),
            "Scale X" => Some((Vector2Track::Scale, Axis2::X)),
            "Scale Y" => Some((Vector2Track::Scale, Axis2::Y)),
            _ => None,
        };
        if let Some((track, axis)) = vector2 {
            return Self::Vector2 { track, axis };
        }
        match key {
            "Rotation" => return Self::Scalar2d(Scalar2dTrack::Rotation),
            "Opacity" => return Self::Scalar2d(Scalar2dTrack::Opacity),
            _ => {}
        }

        for (prefix, track) in [
            ("3D Position ", Vector3Track::Position),
            ("3D Rotation ", Vector3Track::Rotation),
            ("3D Scale ", Vector3Track::Scale),
        ] {
            if let Some(axis) = key.strip_prefix(prefix).and_then(parse_axis3) {
                return Self::Vector3 { track, axis };
            }
        }

        for (prefix, axis) in [("PinX:", Axis2::X), ("PinY:", Axis2::Y)] {
            if let Some(id) = key.strip_prefix(prefix) {
                return Self::PuppetPin { id, axis };
            }
        }

        if let Some(rest) = key.strip_prefix("fxid:") {
            let (base, component) = rest
                .rsplit_once('|')
                .map(|(base, channel)| (base, channel.parse::<usize>().ok()))
                .unwrap_or((rest, None));
            if let Some((effect_id, parameter)) = base.split_once("::") {
                return Self::Effect {
                    effect_id,
                    parameter,
                    component,
                };
            }
        }

        Self::Unknown(key)
    }

    pub(crate) fn key(&self) -> String {
        match self {
            Self::Vector2 { track, axis } => format!(
                "{} {}",
                match track {
                    Vector2Track::Position => "Position",
                    Vector2Track::Scale => "Scale",
                },
                axis.suffix()
            ),
            Self::Scalar2d(track) => match track {
                Scalar2dTrack::Rotation => "Rotation".to_owned(),
                Scalar2dTrack::Opacity => "Opacity".to_owned(),
            },
            Self::Vector3 { track, axis } => format!(
                "3D {} {}",
                match track {
                    Vector3Track::Position => "Position",
                    Vector3Track::Rotation => "Rotation",
                    Vector3Track::Scale => "Scale",
                },
                axis.suffix()
            ),
            Self::PuppetPin { id, axis } => {
                format!("Pin{}:{id}", axis.suffix())
            }
            Self::Effect {
                effect_id,
                parameter,
                component,
            } => match component {
                Some(component) => format!("fxid:{effect_id}::{parameter}|{component}"),
                None => format!("fxid:{effect_id}::{parameter}"),
            },
            Self::Unknown(key) => (*key).to_owned(),
        }
    }
}

impl Axis2 {
    pub(crate) fn index(self) -> usize {
        match self {
            Self::X => 0,
            Self::Y => 1,
        }
    }

    fn suffix(self) -> char {
        match self {
            Self::X => 'X',
            Self::Y => 'Y',
        }
    }
}

impl Axis3 {
    pub(crate) fn index(self) -> usize {
        match self {
            Self::X => 0,
            Self::Y => 1,
            Self::Z => 2,
        }
    }

    fn suffix(self) -> char {
        match self {
            Self::X => 'X',
            Self::Y => 'Y',
            Self::Z => 'Z',
        }
    }
}

fn parse_axis3(value: &str) -> Option<Axis3> {
    match value {
        "X" => Some(Axis3::X),
        "Y" => Some(Axis3::Y),
        "Z" => Some(Axis3::Z),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::GraphProperty;

    #[test]
    fn typed_graph_properties_round_trip_legacy_keys() {
        for key in [
            "Position X",
            "Scale Y",
            "Rotation",
            "Opacity",
            "3D Position Z",
            "3D Rotation X",
            "3D Scale Y",
            "PinX:pin-42",
            "PinY:pin-42",
            "fxid:effect-9::glow_amount",
            "fxid:effect-9::color|2",
            "future-property-id",
        ] {
            assert_eq!(GraphProperty::parse(key).key(), key);
        }

        assert_eq!(
            GraphProperty::parse("Position Y"),
            GraphProperty::Vector2 {
                track: super::Vector2Track::Position,
                axis: super::Axis2::Y,
            }
        );
        assert_eq!(
            GraphProperty::parse("3D Rotation Z"),
            GraphProperty::Vector3 {
                track: super::Vector3Track::Rotation,
                axis: super::Axis3::Z,
            }
        );
        assert_eq!(
            GraphProperty::parse("fxid:effect-9::color|2"),
            GraphProperty::Effect {
                effect_id: "effect-9",
                parameter: "color",
                component: Some(2),
            }
        );
        assert_eq!(
            GraphProperty::parse("PinY:pin-42"),
            GraphProperty::PuppetPin {
                id: "pin-42",
                axis: super::Axis2::Y,
            }
        );
    }
}
