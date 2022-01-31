// use halo2::{
//     circuit::{Chip, Layouter},
//     plonk::{Advice, Column, ConstraintSystem, Error, Selector, Expression},
//     arithmetic::FieldExt,
//     poly::Rotation
// };
// use std::{array, marker::PhantomData};
// use crate::word::{Word};

// pub trait DecomposeInstruction<F: FieldExt> {
//     fn decompose(
//         &self,
//         layouter: impl Layouter<F>,
//         x: Word,
//     ) -> Result<Option<[u8; 4]>, Error>;
// }

// #[derive(Clone, Debug)]
// pub struct DecomposeConfig {
//     pub q_decompose: Selector,
//     pub advice: Column<Advice>,
// }


// #[derive(Clone, Debug)]
// pub struct DecomposeChip<F> {
//     config: DecomposeConfig,
//     _marker: PhantomData<F>,
// }

// impl<F: FieldExt> Chip<F> for DecomposeChip<F> {
//     type Config = DecomposeConfig;
//     type Loaded = ();

//     fn config(&self) -> &Self::Config {
//         &self.config
//     }

//     fn loaded(&self) -> &Self::Loaded {
//         &()
//     }
// }

// // impl<F: FieldExt> UtilitiesInstructions<F> for DecomposeChip<F> {
// //     type Var = NumericCell<F>;
// // }

// impl<F: FieldExt> DecomposeChip<F> {
//     pub fn configure(
//         meta: &mut ConstraintSystem<F>,
//         advice: Column<Advice>,
//     ) -> DecomposeConfig {

//         let q_decompose = meta.selector();

//         let config = DecomposeConfig {
//             q_decompose,
//             advice,
//         };

//         config
//     }

//     pub fn construct(config: DecomposeConfig) -> Self {
//         DecomposeChip {
//             config, 
//             _marker: PhantomData
//         }
//     }
// }

// // impl<F: FieldExt> DecomposeInstruction<F> for DecomposeChip<F> {
// //     fn decompose(
// //         &self, 
// //         mut layouter: impl Layouter<F>,
// //         value: BitWord,
// //     ) -> Result<Option<[u8; 4]>, Error> {
// //         let config = self.config();

// //         layouter.assign_region(
// //             || "decompose", 
// //             |mut region| {
// //             } 
// //         )
// //     }
// // }

// #[cfg(test)]
// mod test {
//     use halo2::{
//         dev::MockProver,
//         pasta::Fp,
//         circuit::{Layouter, SimpleFloorPlanner},
//         plonk::{Advice, Instance, Column, ConstraintSystem, Error},
//         plonk,
//     };

//     use pasta_curves::pallas;

//     use super::{DecomposeChip, DecomposeConfig, DecomposeInstruction};

//     // use crate::utils::{UtilitiesInstructions, NumericCell, Numeric};

//     #[derive(Clone, Debug)]
//     pub struct Config {
//         advice: [Column<Advice>; 3],
//         instance: Column<Instance>,
//         decompose_config: DecomposeConfig
//     }


//     #[derive(Debug, Default)]
//     pub struct Circuit {
//         a: Option<Fp>,
//         b: Option<Fp>,
//         should_decompose: Option<bool>
//     }

//     impl UtilitiesInstructions<pallas::Base> for Circuit {
//         type Var = NumericCell<pallas::Base>;
//     }

//     impl plonk::Circuit<pallas::Base> for Circuit {
//         type Config = Config;
//         type FloorPlanner = SimpleFloorPlanner;

//         fn without_witnesses(&self) -> Self {
//             Self::default()
//         }

//         fn configure(meta: &mut ConstraintSystem<pallas::Base>) -> Self::Config {

//             let advice = [
//                 meta.advice_column(),
//                 meta.advice_column(),
//                 meta.advice_column(),
//             ];

//             let instance = meta.instance_column();
//             meta.enable_equality(instance.into());

//             for advice in advice.iter() {
//                 meta.enable_equality((*advice).into());
//             }

//             let decompose_config = DecomposeChip::configure(meta, advice);

//             Config {
//                 advice, 
//                 instance,
//                 decompose_config
//             }
//         }

//         fn synthesize(
//             &self,
//             config: Self::Config,
//             mut layouter: impl Layouter<pallas::Base>,
//         ) -> Result<(), Error> {
//             let config = config.clone();

//             let a = self.load_private(
//                 layouter.namespace(|| "witness a"),
//                 config.advice[0],
//                 self.a,
//             )?;

//             let decompose_chip = DecomposeChip::<pallas::Base>::construct(config.decompose_config.clone());
//             let decomposeped_pair = decompose_chip.decompose(layouter.namespace(|| "calculate mux"), (a, self.b), self.should_decompose)?;

//             match self.should_decompose.unwrap() {
//                 true => { 
//                     assert_eq!(decomposeped_pair.0.value().unwrap(), self.b.unwrap());
//                     assert_eq!(decomposeped_pair.1.value().unwrap(), self.a.unwrap());
//                 }, 
//                 false => {
//                     assert_eq!(decomposeped_pair.0.value().unwrap(), self.a.unwrap());
//                     assert_eq!(decomposeped_pair.1.value().unwrap(), self.b.unwrap());
//                 }
//             };

//             Ok({})
//         }
//     }

//     #[test]
//     fn decompose_test() {
//         let k = 4;
    
//         let circuit = Circuit {
//             a: Some(Fp::from(1)), 
//             b: Some(Fp::from(2)), 
//             should_decompose: Some(false)
//         };

//         let public_inputs = vec![];
//         let prover = MockProver::run(k, &circuit, vec![public_inputs.clone()]).unwrap();
//         assert_eq!(prover.verify(), Ok(()));
//     }
// }
