use super::*;

pub(super) fn add(comp: &mut Composition, ctr: [f32; 2], _base: f32, lin: InterpolationType) {
    // atmosphere: breathes with the ignition instead of a plain fade
    comp.add_layer(Layer::new(
        "atmos".into(),
        "Atmosphere".into(),
        LayerType::Shape {
            shape_type: ShapeType::Ellipse {
                width: Animatable::new_constant(170.0),
                height: Animatable::new_constant(170.0),
            },
            color: [1.0; 4],
            stroke_color: [0.0; 4],
            stroke_width: 0.0,
            fill_type: ShapeFillType::RadialGradient {
                center: [0.0, 0.0],
                radius: 430.0,
                colors: vec![[0.55, 0.20, 0.04, 0.55], [0.0, 0.0, 0.0, 0.0]],
                stops: vec![0.0, 1.0],
            },
            extrusion_depth: 0.0,
            bevel_depth: 0.0,
        },
        150,
    ));
    {
        let l = comp.layers.last_mut().unwrap();
        l.transform.position = c2(ctr);
        l.transform.opacity = Animatable::new_animated(vec![
            k1(0, 0.0, lin),
            k1(8, 0.0, lin),
            k1(30, 85.0, ez4(E_DRIFT)),
            k1(44, 85.0, lin),
            k1(50, 100.0, ez4(E_SNAP)),
            k1(135, 100.0, lin),
            k1(150, 0.0, lin),
        ]);
        l.transform.scale = Animatable::new_animated(vec![
            k2(0, [100.0, 100.0], lin),
            k2(44, [100.0, 100.0], lin),
            k2(56, [109.0, 109.0], ez4(E_SMOOTH)),
            k2(80, [100.0, 100.0], ez4(E_DRIFT)),
            k2(150, [100.0, 100.0], lin),
        ]);
    }

    // luminous bed: tight warm light under the logo once ignited (two-component
    // falloff with the wide atmosphere above). This carries the mid-tones.
    halo(
        comp,
        "bed",
        120.0,
        300.0,
        [0.65, 0.28, 0.08, 0.6],
        c2([960.0, 430.0]),
        c2([100.0, 100.0]),
        Animatable::new_animated(vec![
            k1(0, 0.0, lin),
            k1(46, 0.0, lin),
            k1(60, 85.0, ez4(E_SMOOTH)),
            k1(135, 85.0, lin),
            k1(150, 0.0, lin),
        ]),
        150,
    );

    // flame skirt: tight hot light clinging to the flame body (permanent).
    halo(
        comp,
        "skirt",
        120.0,
        340.0,
        [0.7, 0.3, 0.08, 0.55],
        c2([960.0, 415.0]),
        c2([100.0, 100.0]),
        Animatable::new_animated(vec![
            k1(0, 0.0, lin),
            k1(48, 0.0, lin),
            k1(62, 55.0, ez4(E_SMOOTH)),
            k1(135, 55.0, lin),
            k1(150, 0.0, lin),
        ]),
        150,
    );

    // energy seed: a faint ember wakes at the ring bottom (f6)
    halo(
        comp,
        "seed",
        30.0,
        150.0,
        [1.0, 0.62, 0.2, 0.9],
        c2([960.0, 733.0]),
        c2([100.0, 100.0]),
        Animatable::new_animated(vec![
            k1(0, 0.0, lin),
            k1(5, 0.0, lin),
            k1(10, 70.0, ez4(E_SMOOTH)),
            k1(16, 0.0, ez4(E_FASTIN)),
            k1(150, 0.0, lin),
        ]),
        150,
    );

    // ignition flash: caused by the converging energy arriving at center
    comp.add_layer(Layer::new(
        "flash".into(),
        "Flash".into(),
        LayerType::Shape {
            shape_type: ShapeType::Ellipse {
                width: Animatable::new_constant(120.0),
                height: Animatable::new_constant(120.0),
            },
            color: [1.0, 0.97, 0.88, 1.0],
            stroke_color: [0.0; 4],
            stroke_width: 0.0,
            fill_type: ShapeFillType::Solid,
            extrusion_depth: 0.0,
            bevel_depth: 0.0,
        },
        150,
    ));
    {
        let l = comp.layers.last_mut().unwrap();
        l.transform.position = c2(ctr);
        l.transform.scale = Animatable::new_animated(vec![
            k2(0, [0.0, 0.0], lin),
            k2(43, [0.0, 0.0], lin),
            k2(46, [30.0, 30.0], ez4(E_SNAP)),
            k2(54, [36.0, 36.0], ez4(E_SMOOTH)),
            k2(66, [0.0, 0.0], ez4(E_DRIFT)),
            k2(150, [0.0, 0.0], lin),
        ]);
        l.transform.opacity = Animatable::new_animated(vec![
            k1(0, 0.0, lin),
            k1(43, 0.0, lin),
            k1(45, 95.0, ez4(E_SNAP)),
            k1(52, 45.0, ez4(E_SMOOTH)),
            k1(66, 0.0, ez4(E_DRIFT)),
            k1(150, 0.0, lin),
        ]);
    }

    // shockwave #1: ignition blast (fast, hot)
    comp.add_layer(Layer::new(
        "shock1".into(),
        "Shockwave1".into(),
        LayerType::Shape {
            shape_type: ShapeType::Ellipse {
                width: Animatable::new_constant(700.0),
                height: Animatable::new_constant(700.0),
            },
            color: [1.0, 0.6, 0.15, 0.0],
            stroke_color: [1.0, 0.75, 0.35, 0.95],
            stroke_width: 7.0,
            fill_type: ShapeFillType::Solid,
            extrusion_depth: 0.0,
            bevel_depth: 0.0,
        },
        150,
    ));
    {
        let l = comp.layers.last_mut().unwrap();
        l.transform.position = c2(ctr);
        l.transform.scale = Animatable::new_animated(vec![
            k2(0, [0.0, 0.0], lin),
            k2(44, [0.0, 0.0], lin),
            k2(62, [9.0, 9.0], ez4(E_SMOOTH)),
            k2(74, [10.0, 10.0], ez4(E_DRIFT)),
            k2(150, [10.0, 10.0], lin),
        ]);
        l.transform.opacity = Animatable::new_animated(vec![
            k1(0, 0.0, lin),
            k1(44, 0.0, lin),
            k1(46, 100.0, ez4(E_SNAP)),
            k1(66, 45.0, ez4(E_SMOOTH)),
            k1(74, 0.0, lin),
            k1(150, 0.0, lin),
        ]);
    }

    // shockwave #2: wing-lock return pulse (slower, thinner, cooler)
    comp.add_layer(Layer::new(
        "shock2".into(),
        "Shockwave2".into(),
        LayerType::Shape {
            shape_type: ShapeType::Ellipse {
                width: Animatable::new_constant(700.0),
                height: Animatable::new_constant(700.0),
            },
            color: [1.0, 0.6, 0.15, 0.0],
            stroke_color: [1.0, 0.55, 0.2, 0.85],
            stroke_width: 4.0,
            fill_type: ShapeFillType::Solid,
            extrusion_depth: 0.0,
            bevel_depth: 0.0,
        },
        150,
    ));
    {
        let l = comp.layers.last_mut().unwrap();
        l.transform.position = c2(ctr);
        l.transform.scale = Animatable::new_animated(vec![
            k2(0, [0.18, 0.18], lin),
            k2(69, [0.18, 0.18], lin),
            k2(92, [8.5, 8.5], ez4(E_SMOOTH)),
            k2(150, [8.5, 8.5], lin),
        ]);
        l.transform.opacity = Animatable::new_animated(vec![
            k1(0, 0.0, lin),
            k1(69, 0.0, lin),
            k1(72, 55.0, ez4(E_SNAP)),
            k1(88, 18.0, ez4(E_SMOOTH)),
            k1(94, 0.0, lin),
            k1(150, 0.0, lin),
        ]);
    }

    // white-hot peak: the whole logo overexposes briefly, then decays
    halo(
        comp,
        "whitepeak",
        240.0,
        600.0,
        [1.0, 0.98, 0.94, 1.0],
        c2([960.0, 440.0]),
        Animatable::new_animated(vec![
            k2(0, [40.0, 40.0], lin),
            k2(80, [40.0, 40.0], lin),
            k2(85, [58.0, 58.0], ez4(E_SNAP)),
            k2(95, [55.0, 55.0], ez4(E_DRIFT)),
            k2(150, [55.0, 55.0], lin),
        ]),
        Animatable::new_animated(vec![
            k1(0, 0.0, lin),
            k1(80, 0.0, lin),
            k1(83, 30.0, ez4(E_SNAP)),
            k1(89, 14.0, ez4(E_SMOOTH)),
            k1(94, 0.0, ez4(E_DRIFT)),
            k1(150, 0.0, lin),
        ]),
        150,
    );
}
