use crate::libs::structs::{
	state::{
		Backend,
		StrataState,
	},
	workspaces::FocusTarget,
};
use smithay::{
	backend::input::{
		self,
		AbsolutePositionEvent,
		Axis,
		AxisSource,
		Event,
		InputBackend,
		InputEvent,
		KeyboardKeyEvent,
		PointerAxisEvent,
		PointerButtonEvent,
		PointerMotionEvent,
	},
	input::{
		keyboard::FilterResult,
		pointer::{
			AxisFrame,
			ButtonEvent,
			MotionEvent,
			RelativeMotionEvent,
		},
	},
	utils::{
		Logical,
		Point,
		SERIAL_COUNTER,
	},
};

impl<BackendData: Backend> StrataState<BackendData> {
	pub fn process_input_event<I: InputBackend>(&mut self, event: InputEvent<I>) {
		match event {
			InputEvent::Keyboard { event, .. } => {
				let serial = SERIAL_COUNTER.next_serial();
				let time = Event::time_msec(&event);

				self.seat.get_keyboard().unwrap().input::<(), _>(
					self,
					event.key_code(),
					event.state(),
					serial,
					time,
					|_, _, _| FilterResult::Forward,
				);
			}
			InputEvent::PointerMotion { event } => {
				let serial = SERIAL_COUNTER.next_serial();
				let delta = (event.delta_x(), event.delta_y()).into();
				self.pointer_location += delta;
				self.pointer_location = self.clamp_coords(self.pointer_location);

				let under = self.surface_under();

				self.set_input_focus_auto();

				if let Some(ptr) = self.seat.get_pointer() {
					ptr.motion(
						self,
						under.clone(),
						&MotionEvent {
							location: self.pointer_location,
							serial,
							time: event.time_msec(),
						},
					);

					ptr.relative_motion(
						self,
						under,
						&RelativeMotionEvent {
							delta,
							delta_unaccel: event.delta_unaccel(),
							utime: event.time(),
						},
					)
				}
			}
			InputEvent::PointerMotionAbsolute { event, .. } => {
				let output = self.workspaces.current().outputs().next().unwrap().clone();
				let output_geo = self.workspaces.current().output_geometry(&output).unwrap();
				let pos = event.position_transformed(output_geo.size) + output_geo.loc.to_f64();
				let serial = SERIAL_COUNTER.next_serial();
				let pointer = self.seat.get_pointer().unwrap();
				self.pointer_location = self.clamp_coords(pos);
				let under = self.surface_under();
				self.set_input_focus_auto();
				pointer.motion(
					self,
					under,
					&MotionEvent { location: pos, serial, time: event.time_msec() },
				);
			}
			InputEvent::PointerButton { event, .. } => {
				let pointer = self.seat.get_pointer().unwrap();
				let serial = SERIAL_COUNTER.next_serial();
				let button = event.button_code();
				let button_state = event.state();
				self.set_input_focus_auto();
				pointer.button(
					self,
					&ButtonEvent { button, state: button_state, serial, time: event.time_msec() },
				);
			}
			InputEvent::PointerAxis { event, .. } => {
				let horizontal_amount =
					event.amount(input::Axis::Horizontal).unwrap_or_else(|| {
						event.amount_discrete(input::Axis::Horizontal).unwrap_or(0.0) * 3.0
					});
				let vertical_amount = event.amount(input::Axis::Vertical).unwrap_or_else(|| {
					event.amount_discrete(input::Axis::Vertical).unwrap_or(0.0) * 3.0
				});
				let horizontal_amount_discrete = event.amount_discrete(input::Axis::Horizontal);
				let vertical_amount_discrete = event.amount_discrete(input::Axis::Vertical);

				{
					let mut frame = AxisFrame::new(event.time_msec()).source(event.source());
					if horizontal_amount != 0.0 {
						frame = frame.value(Axis::Horizontal, horizontal_amount);
						if let Some(discrete) = horizontal_amount_discrete {
							frame = frame.discrete(Axis::Horizontal, discrete as i32);
						}
					} else if event.source() == AxisSource::Finger {
						frame = frame.stop(Axis::Horizontal);
					}
					if vertical_amount != 0.0 {
						frame = frame.value(Axis::Vertical, vertical_amount);
						if let Some(discrete) = vertical_amount_discrete {
							frame = frame.discrete(Axis::Vertical, discrete as i32);
						}
					} else if event.source() == AxisSource::Finger {
						frame = frame.stop(Axis::Vertical);
					}
					self.seat.get_pointer().unwrap().axis(self, frame);
				}
			}
			_ => {}
		}
	}
	fn clamp_coords(&self, pos: Point<f64, Logical>) -> Point<f64, Logical> {
		if self.workspaces.current().outputs().next().is_none() {
			return pos;
		}

		let (pos_x, pos_y) = pos.into();
		let (max_x, max_y) = self
			.workspaces
			.current()
			.output_geometry(self.workspaces.current().outputs().next().unwrap())
			.unwrap()
			.size
			.into();
		let clamped_x = pos_x.max(0.0).min(max_x as f64);
		let clamped_y = pos_y.max(0.0).min(max_y as f64);
		(clamped_x, clamped_y).into()
	}

	pub fn set_input_focus(&mut self, target: FocusTarget) {
		let keyboard = self.seat.get_keyboard().unwrap();
		let serial = SERIAL_COUNTER.next_serial();
		keyboard.set_focus(self, Some(target), serial);
	}

	pub fn set_input_focus_auto(&mut self) {
		let under = self.surface_under();
		if let Some(d) = under {
			self.set_input_focus(d.0);
		}
	}
}
