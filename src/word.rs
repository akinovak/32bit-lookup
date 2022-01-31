use halo2::{
    arithmetic::FieldExt,
    circuit::{Layouter, Region, AssignedCell},
    plonk::{Column, Advice, Error, Instance, Assigned},
    circuit
};

use pasta_curves::pallas;

#[derive(Clone, Debug, Copy, Default)]
pub struct Word(u32);

impl Word {
    fn new(x: u32) -> Self {
        return Word(x) 
    }

    fn decompose_4(&self) -> [u8; 4] {
        self.to_le_bytes()
    }

    fn compose(chunks: [u8; 4]) -> Self {
        return Word(u32::from_le_bytes(chunks))
    }
}

impl std::ops::Deref for Word {
    type Target = u32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, Debug)]
pub struct AssignedWord(AssignedCell<Word, pallas::Base>);

impl AssignedWord {
    fn new(assigned_cell: AssignedCell<Word, pallas::Base>) -> Self {
        AssignedWord(assigned_cell)
    }

    fn value_word(&self) -> Option<u32> {
        self.0.value().map(|v| (*v).0)
    }
}

impl From<&Word> for Assigned<pallas::Base> {
    fn from(word: &Word) -> Assigned<pallas::Base> {
        pallas::Base::from(word.0 as u64).into()
    }
}


impl AssignedWord {
    fn assign_word(
        mut layouter: impl Layouter<pallas::Base>,
        column: Column<Advice>,
        value: Option<Word>,
    ) -> Result<AssignedWord, Error> {
        layouter.assign_region(
            || "witness word",
            |mut region| {
                let assigned = region.assign_advice(
                    || "witness",
                    column,
                    0,
                    || value.ok_or(Error::Synthesis),
                )?;
                Ok(AssignedWord::new(assigned))
            },
        )
    }
}

#[derive(Clone, Debug, Copy, Default)]
pub struct Chunk(u8);

impl Chunk {
    fn new(x: u8) -> Self {
        return Chunk(x) 
    }
}

impl std::ops::Deref for Chunk {
    type Target = u8;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, Debug)]
pub struct AssignedChunk(AssignedCell<Chunk, pallas::Base>);

impl AssignedChunk {
    fn new(assigned_cell: AssignedCell<Chunk, pallas::Base>) -> Self {
        AssignedChunk(assigned_cell)
    }

    fn value_word(&self) -> Option<u8> {
        self.0.value().map(|v| (*v).0)
    }
}

impl From<&Chunk> for Assigned<pallas::Base> {
    fn from(chunk: &Chunk) -> Assigned<pallas::Base> {
        pallas::Base::from(chunk.0 as u64).into()
    }
}


impl AssignedChunk {
    fn assign_chunk(
        mut layouter: impl Layouter<pallas::Base>,
        column: Column<Advice>,
        value: Option<Chunk>,
    ) -> Result<AssignedChunk, Error> {
        layouter.assign_region(
            || "witness word",
            |mut region| {
                let assigned = region.assign_advice(
                    || "witness",
                    column,
                    0,
                    || value.ok_or(Error::Synthesis),
                )?;
                Ok(AssignedChunk::new(assigned))
            },
        )
    }
}


#[cfg(test)]
mod test {

    use halo2::{
        circuit::{Layouter, SimpleFloorPlanner},
        plonk::{Advice, Instance, Column, ConstraintSystem, Error},
        plonk,
        pasta::Fp,
        dev::MockProver,
    };
    use std::marker::PhantomData;
    use pasta_curves::pallas;

    use super::{Word, AssignedWord};

    #[derive(Clone, Debug)]
    pub struct Config {
        advice: [Column<Advice>; 4],
        // instance: Column<Instance>,
    }

    #[derive(Clone, Debug, Default)]
    pub struct Circuit {
        value: Option<Word>
    }

    // impl UtilitiesInstructions<pallas::Base> for Circuit {
    //     type Var = NumericCell<pallas::Base>;
    // }

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
                meta.advice_column()
            ];

            // let instance = meta.instance_column();
            // meta.enable_equality(instance.into());

            for advice in advice.iter() {
                meta.enable_equality((*advice).into());
            }

            Config {
                advice, 
                // instance,
            }
        }

        fn synthesize(
            &self,
            config: Self::Config,
            mut layouter: impl Layouter<pallas::Base>,
        ) -> Result<(), Error> {
            let config = config.clone();

            let assigned = AssignedWord::assign_word(layouter.namespace(|| "witness value"), config.advice[0], self.value).unwrap();
            println!("{:?}", assigned.value_word());
            Ok(())
        }
    }

    #[test]
    fn assign_word() {
        let value = Word::new(5);
        let circuit = Circuit {
            value: Some(value)
        };
        let k = 4;
        let prover = MockProver::run(k, &circuit, vec![]).unwrap();
        assert_eq!(prover.verify(), Ok(()));
    }
}