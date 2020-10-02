use core::marker::PhantomData;
use embedded_hal::digital::v2::OutputPin;
use embedded_hal::adc::OneShot;
use embedded_hal::adc::Channel;

pub struct Gp2y1014au<PinLed, OneShotReader, Adc, Word, PinData>
where 
    PinLed: OutputPin, 
    OneShotReader: OneShot<Adc, Word, PinData>,
    PinData: Channel<Adc>
{
    pin_led: PinLed,
    one_shot_reader: OneShotReader,
    pin_data: PinData,
    _unused: PhantomData<Adc>,
    _unused2: PhantomData<Word>,
   
}

pub enum Error<OutputError, AdcError> {
    LedError(OutputError),
    ReadError(AdcError)
}

impl <PinLed, OneShotReader, Adc, Word, PinData>  Gp2y1014au <PinLed, OneShotReader, Adc, Word, PinData>
where 
    PinLed: OutputPin, 
    OneShotReader: OneShot<Adc, Word, PinData>,
    PinData: Channel<Adc> ,
{
    /// Creates a new instance of the Gp2y1014au dust sensor
    /// 
    /// # Arguments
    ///
    /// * `pin_led`  - The pin connected to the led for the sensor.
    /// * `pin_data` - The pin connected to data/out on the sensor.
    /// * `one_shot_reader` - A structure that implements "embedded_hal::adc::OneShot"
    ///
    /// # Example
    ///
    /// ```ignore
    /// use stm32f4xx_hal::
    ///     adc::{
    ///       Adc,
    ///       config::AdcConfig
    ///     };
    /// 
    /// // ... 
    /// let pc1_led = gpioc.pc1.into_push_pull_output();
    /// let pc0_out = gpioc.pc0.into_analog();
    /// let mut adc = Adc::adc1(board_peripherals.ADC1, true, AdcConfig::default());
    /// let mut reader = Gp2y1014au::new(pc1_led, pc0_out, adc);
    /// ```
    pub fn new(pin_led: PinLed, pin_data: PinData, one_shot_reader: OneShotReader) -> Self {
        Self {
            pin_led,
            one_shot_reader,
            pin_data,
            _unused: PhantomData,
            _unused2: PhantomData,
        }
    }

    /// Reads the pin state. Returns back `Word` which varies based on your HAL implementation.
    ///
    /// The error types returned back from this will either be `Error::LedError` or `Error::ReadError`.
    ///
    /// * `Error::ReadError` - Implies the OneShot::read function failed for some reason. `nb::Error::WouldBlock`
    /// is already handled in the code as a loop.
    /// * `Error::LedError` - Implies the pin for the LED was either failed to be set low or high respectively. 
    /// This error indicates you should probably discard the result and call the method again. 
    pub fn read(&mut self) -> core::result::Result<Word, Error<PinLed::Error, OneShotReader::Error>> {
        match self.pin_led.set_low() {
            Ok(()) => (),
            Err(error) => return Err(Error::LedError(error)),
        };
        let result;
        loop { 
            let read_result = self.one_shot_reader.read(&mut self.pin_data);

            match read_result {
                Ok(word) => {
                    result = Ok(word);
                    break
                }
                Err(nb::Error::Other(failed)) => {
                    result = Err(Error::ReadError(failed));
                    break
                }
                Err(nb::Error::WouldBlock) => continue
            };
        }
        match self.pin_led.set_high() {
            Ok(()) => (),
            Err(error) => return Err(Error::LedError(error)),
        };

        result
    }    

    /// Returns back the pins and reader used to construct the sensor.
    /// This function consumes self.
    pub fn split(self) -> (PinLed, PinData, OneShotReader) {
        (self.pin_led, self.pin_data, self.one_shot_reader)
    }

    
}

#[cfg(test)]
mod tests {
    use core::marker::PhantomData;
    use embedded_hal::digital::v2::OutputPin;
    use embedded_hal::adc::OneShot;
    use embedded_hal::adc::Channel;
    struct BadState;
    struct GoodState;
    struct TestAdc {
        _garbage: bool 
    }

    impl TestAdc {
        fn new() -> Self {
            Self {_garbage: true}
        }
    }
    struct TestAnalogPin<STATE> {
        _unused: PhantomData<STATE>
    }
    impl <STATE> TestAnalogPin<STATE> {
        fn new() -> Self {
            Self { _unused: PhantomData }
        }
    }
    
    struct TestOutputPin<STATE> {
        _unused: PhantomData<STATE>
    }


    impl <STATE> TestOutputPin<STATE> {
        fn new() -> Self { 
            Self { _unused: PhantomData }
        }
    }

    impl OutputPin for TestOutputPin<GoodState> {
        type Error = ();
        fn set_high(&mut self) -> Result<(), Self::Error> {
            Ok(())
        }
        fn set_low(&mut self) -> Result<(), Self::Error> {
            Ok(())
        }
    }
    impl OutputPin for TestOutputPin<BadState> {
        type Error = ();
        fn set_high(&mut self) -> Result<(), Self::Error> {
            Err(())
        }
        fn set_low(&mut self) -> Result<(), Self::Error> {
            Err(())
        }
    }
    impl <STATE> Channel<TestAdc> for TestAnalogPin<STATE> {
        type ID = u8;
        fn channel() -> Self::ID {
            return 1;
        }
    }

    impl OneShot<TestAdc, u8, TestAnalogPin<GoodState>> for TestAdc {
        type Error = ();
        fn read(&mut self, _: &mut TestAnalogPin<GoodState>) -> nb::Result<u8, ()> {
            Ok(10u8)
        }
    }
    impl OneShot<TestAdc, u8, TestAnalogPin<BadState>> for TestAdc {
        type Error = ();
        fn read(&mut self, _: &mut TestAnalogPin<BadState>) -> nb::Result<u8, ()> {
            Err(nb::Error::Other(()))
        }
    }
    #[test]
    fn read_returns_value_when_no_errors_present() {
        let led_pin: TestOutputPin<GoodState> = TestOutputPin::new();
        let data_pin: TestAnalogPin<GoodState> = TestAnalogPin::new();
        let test_adc: TestAdc = TestAdc::new();
        let mut sensor = crate::sensor::Gp2y1014au::new(led_pin, data_pin, test_adc);
        assert_eq!(10u8, sensor.read().ok().unwrap() );
    }

    #[test]
    fn read_returns_error_when_one_shot_read_fails() {
        let led_pin: TestOutputPin<GoodState> = TestOutputPin::new();
        let data_pin: TestAnalogPin<BadState> = TestAnalogPin::new();
        let test_adc: TestAdc = TestAdc::new();
        let mut sensor = crate::sensor::Gp2y1014au::new(led_pin, data_pin, test_adc);
        sensor.read().expect_err("Expected this function to error");
    }
    

    // struct 
}