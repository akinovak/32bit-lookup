/* 
This chip should determine which value should be left and which right input to hasher
based on leaf position at current level

This chip considers that left value is already witnessed:
first layer -> leaf is witnessed as private input
ith layer -> left inter root is witnessed after hash layer
*/
use halo2::{
    circuit::{Chip, Layouter},
    plonk::{Advice, Column, ConstraintSystem, Error, Selector, Expression},
    arithmetic::FieldExt,
    poly::Rotation
};
use std::{array, marker::PhantomData};
use pasta_curves::pallas;
use crate::word::{AssignedWord, AssignedChunk, Word, Chunk};

use crate::gates::{Gate};

pub trait DecomposeInstruction<F: FieldExt> {
    fn decompose(
        &self,
        layouter: impl Layouter<pallas::Base>,
        value: AssignedWord,
    ) -> Result<(AssignedChunk, AssignedChunk, AssignedChunk, AssignedChunk), Error>;
}

#[derive(Clone, Debug)]
pub struct DecomposeConfig {
    pub q_decompose: Selector,
    pub advice: [Column<Advice>; 3],
}


#[derive(Clone, Debug)]
pub struct DecomposeChip<F> {
    config: DecomposeConfig,
    _marker: PhantomData<F>,
}

impl<F: FieldExt> Chip<F> for DecomposeChip<F> {
    type Config = DecomposeConfig;
    type Loaded = ();

    fn config(&self) -> &Self::Config {
        &self.config
    }

    fn loaded(&self) -> &Self::Loaded {
        &()
    }
}


impl<F: FieldExt> DecomposeChip<F> {
    pub fn configure(
        meta: &mut ConstraintSystem<F>,
        advice: [Column<Advice>; 3],
    ) -> DecomposeConfig {
        // we must enable equality so that copy can work
        for advice in advice.iter() {
            meta.enable_equality((*advice).into());
        }

        let q_decompose = meta.selector();

        let config = DecomposeConfig {
            q_decompose,
            advice
        };

        meta.create_gate("constraint decompose", |meta| {
            let q_decompose = meta.query_selector(q_decompose);
            
            let x = meta.query_advice(config.advice[0], Rotation::cur());

            let x0 = meta.query_advice(config.advice[1], Rotation::cur());
            let x1 = meta.query_advice(config.advice[2], Rotation::cur());
            let x2 = meta.query_advice(config.advice[0], Rotation::next());
            let x3 = meta.query_advice(config.advice[1], Rotation::next());

            Gate::decompose_32_to_8(
                q_decompose,
                x,
                x0, 
                x1, 
                x2, 
                x3
            )

        });

        config
    }

    pub fn construct(config: DecomposeConfig) -> Self {
        DecomposeChip {
            config, 
            _marker: PhantomData
        }
    }
}

impl<F: FieldExt> DecomposeInstruction<F> for DecomposeChip<F> {
    fn decompose(
        &self, 
        mut layouter: impl Layouter<pallas::Base>,
        value: AssignedWord
    ) -> Result<(AssignedChunk, AssignedChunk, AssignedChunk, AssignedChunk), Error> {
        let config = self.config();

        layouter.assign_region(
            || "decompose", 
            |mut region| {
                let mut row_offset = 0;
                config.q_decompose.enable(&mut region, 0)?;

                //make sure that root from previous level is equal to left
                // let left = pair.0.copy(|| "copy left", &mut region, config.left, row_offset)?;

                let value = {
                    let assigned = region.assign_advice(
                        || "witness right",
                        config.advice[0],
                        row_offset,
                        || value.value_word().ok_or(Error::Synthesis),
                    )?;
                    AssignedWord::new(assigned)
                };

                let decomposed = value.value_word().map(|word| word.decompose_4());

                let x0 = {
                    let assigned = region.assign_advice(
                        || "assign x0",
                        config.advice[1],
                        row_offset,
                        || decomposed.map(|decomposed| decomposed[0]).ok_or(Error::Synthesis),
                        // || value.value_word().ok_or(Error::Synthesis),
                    )?;

                    AssignedChunk::new(assigned)
                };

                let x1 = {
                    let assigned = region.assign_advice(
                        || "assign x1",
                        config.advice[2],
                        row_offset,
                        || decomposed.map(|decomposed| decomposed[1]).ok_or(Error::Synthesis),
                    )?;

                    AssignedChunk::new(assigned)
                };

                row_offset += 1;
                let x2 = {
                    let assigned = region.assign_advice(
                        || "assign x2",
                        config.advice[0],
                        row_offset,
                        || decomposed.map(|decomposed| decomposed[2]).ok_or(Error::Synthesis),
                        // || value.value_word().ok_or(Error::Synthesis),

                    )?;

                    AssignedChunk::new(assigned)
                };

                let x3 = {
                    let assigned = region.assign_advice(
                        || "assign x3",
                        config.advice[1],
                        row_offset,
                        || decomposed.map(|decomposed| decomposed[3]).ok_or(Error::Synthesis),
                        // || value.value_word().ok_or(Error::Synthesis),
                    )?;

                    AssignedChunk::new(assigned)
                };

                Ok((x0, x1, x2, x3))
            } 
        )
    }
}

#[cfg(test)]
mod test {
    use halo2::{
        dev::MockProver,
        pasta::Fp,
        circuit::{Layouter, SimpleFloorPlanner},
        plonk::{Advice, Instance, Column, ConstraintSystem, Error},
        plonk,
    };

    use pasta_curves::pallas;

    use super::{DecomposeChip, DecomposeConfig, DecomposeInstruction};

    use crate::word::{AssignedWord, Word};

    #[derive(Clone, Debug)]
    pub struct Config {
        advice: [Column<Advice>; 3],
        instance: Column<Instance>,
        decompose_config: DecomposeConfig
    }


    #[derive(Debug, Default)]
    pub struct Circuit {
        a: Option<Word>
    }

    impl plonk::Circuit<pallas::Base> for Circuit {
        type Config = Config;
        type FloorPlanner = SimpleFloorPlanner;

        fn without_witnesses(&self) -> Self {
            Self::default()
        }

        fn configure(meta: &mut ConstraintSystem<pallas::Base>) -> Self::Config {

            let advice = [
                meta.advice_column(),
                meta.advice_column(),
                meta.advice_column(),
            ];

            let instance = meta.instance_column();
            // meta.enable_equality(instance.into());

            for advice in advice.iter() {
                meta.enable_equality((*advice).into());
            }

            let decompose_config = DecomposeChip::configure(meta, advice);

            Config {
                advice, 
                instance,
                decompose_config
            }
        }

        fn synthesize(
            &self,
            config: Self::Config,
            mut layouter: impl Layouter<pallas::Base>,
        ) -> Result<(), Error> {
            let config = config.clone();

            let a = AssignedWord::assign_word(layouter.namespace(|| "witness value"), config.advice[0], self.a)?;

            let decompose_chip = DecomposeChip::<pallas::Base>::construct(config.decompose_config.clone());
            decompose_chip.decompose(layouter.namespace(|| "decompose"), a)?;

            Ok({})
        }
    }

    #[test]
    fn decompose_test() {
        let k = 4;
    
        let circuit = Circuit {
            a: Some(Word::new(5))
        };

        let public_inputs = vec![];
        let prover = MockProver::run(k, &circuit, vec![public_inputs.clone()]).unwrap();
        assert_eq!(prover.verify(), Ok(()));
    }
}
