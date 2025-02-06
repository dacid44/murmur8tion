use criterion::{black_box, criterion_group, criterion_main, Criterion};
use murmur8tion::{
    hardware::{Chip8, DynamicMachine, Machine},
    model::{CosmacVip, Model},
    screen::{CosmacVipScreen, DynamicScreen, Screen},
};

const TEST_ROM: &[u8] = &[0x12, 0x00, 0x00, 0x00];
const DRAW_TEST_ROM: &[u8] = &[0xA2, 0x06, 0xD0, 0x02, 0x12, 0x02, 0x00, 0x00];

pub fn dynamic_machine_dispatch(c: &mut Criterion) {
    let mut machine = black_box(DynamicMachine::new_cosmac_vip(
        CosmacVip::default(),
        TEST_ROM,
    ));
    let mut draw_machine = black_box(DynamicMachine::new_cosmac_vip(
        CosmacVip::default(),
        DRAW_TEST_ROM,
    ));
    let _ = draw_machine.tick();
    c.bench_function("dynamic machine dispatch", |b| {
        b.iter(|| {
            let _ = machine.tick();
        })
    });
    c.bench_function("dynamic machine dispatch draw", |b| {
        b.iter(|| {
            let _ = machine.tick();
            let _ = machine.tick();
        })
    });
}

pub fn dyn_model_dispatch(c: &mut Criterion) {
    let mut machine: Chip8<Box<dyn Model>, dyn Screen> = black_box(Chip8::new(
        Box::new(CosmacVip::default()),
        Box::<CosmacVipScreen>::default(),
        TEST_ROM,
    ));
    let mut draw_machine: Chip8<Box<dyn Model>, dyn Screen> = black_box(Chip8::new(
        Box::new(CosmacVip::default()),
        Box::<CosmacVipScreen>::default(),
        DRAW_TEST_ROM,
    ));
    let _ = draw_machine.tick();
    c.bench_function("dyn model dispatch", |b| {
        b.iter(|| {
            let _ = machine.tick();
        })
    });
    c.bench_function("dyn model dispatch draw", |b| {
        b.iter(|| {
            let _ = machine.tick();
            let _ = machine.tick();
        })
    });
}

pub fn dyn_model_enum_screen(c: &mut Criterion) {
    let mut machine: Chip8<Box<dyn Model>, DynamicScreen> = black_box(Chip8::new(
        Box::new(CosmacVip::default()),
        DynamicScreen::new_cosmac_vip(),
        TEST_ROM,
    ));
    let mut draw_machine: Chip8<Box<dyn Model>, DynamicScreen> = black_box(Chip8::new(
        Box::new(CosmacVip::default()),
        DynamicScreen::new_cosmac_vip(),
        DRAW_TEST_ROM,
    ));
    let _ = draw_machine.tick();
    c.bench_function("dyn model enum screen", |b| {
        b.iter(|| {
            let _ = machine.tick();
        })
    });
    c.bench_function("dyn model enum screen draw", |b| {
        b.iter(|| {
            let _ = machine.tick();
            let _ = machine.tick();
        })
    });
}

pub fn dyn_machine(c: &mut Criterion) {
    let mut machine: Box<dyn Machine> = black_box(Box::new(Chip8::new(
        CosmacVip::default(),
        Box::<CosmacVipScreen>::default(),
        TEST_ROM,
    )));
    let mut draw_machine: Box<dyn Machine> = black_box(Box::new(Chip8::new(
        CosmacVip::default(),
        Box::<CosmacVipScreen>::default(),
        DRAW_TEST_ROM,
    )));
    let _ = draw_machine.tick();
    c.bench_function("dyn machine", |b| {
        b.iter(|| {
            let _ = machine.tick();
        })
    });
    c.bench_function("dyn machine draw", |b| {
        b.iter(|| {
            let _ = machine.tick();
            let _ = machine.tick();
        })
    });
}

criterion_group!(
    benches,
    dynamic_machine_dispatch,
    dyn_model_dispatch,
    dyn_model_enum_screen,
    dyn_machine,
);
criterion_main!(benches);
