use super::*;

pub(super) fn add(comp: &mut Composition, ctr: [f32; 2], base: f32, lin: InterpolationType) {
    // sideways release streaks: flame expansion fires energy to both sides.
    // The wings deploy BECAUSE these streaks arrive (cause -> effect).
    for (side, x0) in [("l", 960.0), ("r", 960.0)] {
        let dir = if side == "l" { -1.0 } else { 1.0 };
        let id = format!("streak_{side}");
        let pts = vec![
            [0.0, -3.0],
            [dir * 46.0, -3.0],
            [dir * 46.0, 3.0],
            [0.0, 3.0],
        ];
        let d = if side == "l" { 0 } else { 2 };
        apart(
            comp,
            &id,
            pts.clone(),
            sharp(&pts),
            [1.0, 0.8, 0.45, 1.0],
            ShapeFillType::Solid,
            c2([x0, 444.0]),
            Animatable::new_animated(vec![
                k2(0, [0.0, base], lin),
                k2(52 + d, [0.0, base], lin),
                k2(59 + d, [base * 1.1, base], ez4(E_FASTIN)),
                k2(150, [base * 1.1, base], lin),
            ]),
            Animatable::new_animated(vec![
                k1(0, 0.0, lin),
                k1(52 + d, 0.0, lin),
                k1(55 + d, 90.0, ez4(E_SNAP)),
                k1(62 + d, 0.0, ez4(E_SMOOTH)),
                k1(150, 0.0, lin),
            ]),
            150,
        );
    }

    // wings materialize at the streak impacts: short travel, then opacity pops
    // at arrival (no long dark slides). Dark mass first, bright blade unfolds
    // 3f later with a wider swing (follow-through). Left leads by 2f.
    let deploy = |from_x: f32, t0: u32, rot0: f32| {
        (
            Animatable::new_animated(vec![
                k2(0, [ctr[0] + from_x, ctr[1]], lin),
                k2(t0, [ctr[0] + from_x, ctr[1]], lin),
                k2(t0 + 12, [ctr[0] + from_x * 0.12, ctr[1]], ez4(E_SNAP)),
                k2(t0 + 16, ctr, ez4(E_SMOOTH)),
                k2(150, ctr, lin),
            ]),
            Animatable::new_animated(vec![
                k1(0, rot0, lin),
                k1(t0, rot0, lin),
                k1(t0 + 12, -rot0 * 0.22, ez4(E_SNAP)),
                k1(t0 + 18, 0.0, ez4(E_SMOOTH)),
                k1(150, 0.0, lin),
            ]),
        )
    };
    let wing_op = |t0: u32| {
        Animatable::new_animated(vec![
            k1(0, 0.0, lin),
            k1(t0 + 8, 0.0, lin),
            k1(t0 + 12, 100.0, ez4(E_SNAP)),
            k1(135, 100.0, lin),
            k1(150, 0.0, lin),
        ])
    };
    let unfold = |t0: u32| {
        Animatable::new_animated(vec![
            k2(0, [base * 0.35, base], lin),
            k2(t0 + 8, [base * 0.35, base], lin),
            k2(t0 + 13, [base * 1.12, base], ez4(E_SNAP)),
            k2(t0 + 18, [base, base], ez4(E_SMOOTH)),
            k2(150, [base, base], lin),
        ])
    };
    let wd = wing_dark_l_pts();
    let (wp, wr) = deploy(-225.0, 56, -16.0);
    apart(
        comp,
        "wing_dl",
        wd.clone(),
        sharp(&wd),
        [0.08, 0.10, 0.16, 1.0],
        ShapeFillType::Solid,
        wp,
        c2([base, base]),
        wing_op(56),
        150,
    );
    comp.layers.last_mut().unwrap().transform.rotation = wr;
    let wdr = mirror(&wd);
    let (wp, wr) = deploy(225.0, 58, 16.0);
    apart(
        comp,
        "wing_dr",
        wdr.clone(),
        sharp(&wdr),
        [0.08, 0.10, 0.16, 1.0],
        ShapeFillType::Solid,
        wp,
        c2([base, base]),
        wing_op(58),
        150,
    );
    comp.layers.last_mut().unwrap().transform.rotation = wr;
    let wb = wing_blade_l_pts();
    let (wp, wr) = deploy(-225.0, 59, -23.0);
    apart(
        comp,
        "wing_l",
        wb.clone(),
        sharp(&wb),
        [0.96, 0.95, 0.90, 1.0],
        ShapeFillType::Solid,
        wp,
        unfold(59),
        wing_op(59),
        150,
    );
    comp.layers.last_mut().unwrap().transform.rotation = wr;
    let wbr = mirror(&wb);
    let (wp, wr) = deploy(225.0, 61, 23.0);
    apart(
        comp,
        "wing_r",
        wbr.clone(),
        sharp(&wbr),
        [0.96, 0.95, 0.90, 1.0],
        ShapeFillType::Solid,
        wp,
        unfold(61),
        wing_op(61),
        150,
    );
    comp.layers.last_mut().unwrap().transform.rotation = wr;

    // wing-root impact glows: reaction to each lock (left f72, right f74)
    halo(
        comp,
        "root_l",
        40.0,
        200.0,
        [1.0, 0.8, 0.5, 1.0],
        c2([792.0, 445.0]),
        c2([100.0, 100.0]),
        Animatable::new_animated(vec![
            k1(0, 0.0, lin),
            k1(68, 0.0, lin),
            k1(72, 100.0, ez4(E_SNAP)),
            k1(80, 0.0, ez4(E_SMOOTH)),
            k1(150, 0.0, lin),
        ]),
        150,
    );
    halo(
        comp,
        "root_r",
        40.0,
        200.0,
        [1.0, 0.8, 0.5, 1.0],
        c2([1128.0, 445.0]),
        c2([100.0, 100.0]),
        Animatable::new_animated(vec![
            k1(0, 0.0, lin),
            k1(70, 0.0, lin),
            k1(74, 100.0, ez4(E_SNAP)),
            k1(82, 0.0, ez4(E_SMOOTH)),
            k1(150, 0.0, lin),
        ]),
        150,
    );

    // shield rises heavy: slow-in, small overshoot bounce, settles
    let rise = Animatable::new_animated(vec![
        k2(0, [ctr[0], ctr[1] + 330.0], lin),
        k2(60, [ctr[0], ctr[1] + 330.0], lin),
        k2(78, [ctr[0], ctr[1] - 7.0], ez4(E_HEAVY)),
        k2(84, ctr, ez4(E_SMOOTH)),
        k2(150, ctr, lin),
    ]);
    let shield_op = Animatable::new_animated(vec![
        k1(0, 0.0, lin),
        k1(60, 0.0, lin),
        k1(70, 100.0, ez4(E_SMOOTH)),
        k1(135, 100.0, lin),
        k1(150, 0.0, lin),
    ]);
    let sl = shield_l_pts();
    apart(
        comp,
        "cube_l",
        sl.clone(),
        sharp(&sl),
        [0.757, 0.722, 0.655, 1.0],
        ShapeFillType::Solid,
        rise.clone(),
        c2([base, base]),
        shield_op.clone(),
        150,
    );
    let sr = cube_r_pts();
    apart(
        comp,
        "cube_r",
        sr.clone(),
        sharp(&sr),
        [0.122, 0.141, 0.169, 1.0],
        ShapeFillType::Solid,
        rise,
        c2([base, base]),
        shield_op,
        150,
    );
}
