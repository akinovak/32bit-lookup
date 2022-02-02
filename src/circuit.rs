use halo2::{
    circuit::{Layouter, SimpleFloorPlanner},
    plonk::{Advice, Instance, Column, ConstraintSystem, Error},
    plonk,
    pasta::Fp
};
use std::marker::PhantomData;
use pasta_curves::pallas;


use crate::word::{Word, Chunk, AssignedChunk, AssignedWord};
use crate::gadget::{
    decompose::{DecomposeChip, DecomposeConfig, DecomposeInstruction},
    xor::{TableChip, TableConfig}
};


#[derive(Clone, Debug)]
pub struct Config {
    advice: [Column<Advice>; 6],
    // instance: Column<Instance>,
    table_config: TableConfig, 
    decompose_config: DecomposeConfig
}


#[derive(Clone, Debug, Default)]
pub struct Circuit {
    x: Option<Word>,
    y: Option<Word>
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
            meta.advice_column(),
            meta.advice_column(),
            meta.advice_column(),
        ];

        // let instance = meta.instance_column();
        // meta.enable_equality(instance.into());

        for advice in advice.iter() {
            meta.enable_equality((*advice).into());
        }

        let table_config = TableChip::configure(meta, advice[3], advice[4], advice[5]);
        let decompose_config = DecomposeChip::configure(meta, advice[0..3].try_into().unwrap());

        Config {
            advice, 
            // instance,
            table_config,
            decompose_config
        }
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<pallas::Base>,
    ) -> Result<(), Error> {
        let config = config.clone();

        TableChip::load(config.table_config.clone(), &mut layouter)?;
        let table_chip = TableChip::<pallas::Base>::construct(config.table_config.clone());
        let decompose_chip = DecomposeChip::<pallas::Base>::construct(config.decompose_config.clone());

        let x = AssignedWord::assign_word(layouter.namespace(|| "witness value"), config.advice[0], self.x).unwrap();
        let y = AssignedWord::assign_word(layouter.namespace(|| "witness value"), config.advice[0], self.y).unwrap();

        let (x0, x1, x2, x3) = decompose_chip.decompose(layouter.namespace(|| "decompose x"), x)?;
        let (y0, y1, y2, y3) = decompose_chip.decompose(layouter.namespace(|| "decompose y"), y)?;

        let z0 = x0.value_chunk().zip(y0.value_chunk())
                .map(|(x0, y0)| Chunk::new(*x0 ^ *y0));

        let z1 = x1.value_chunk().zip(y1.value_chunk())
                .map(|(x1, y1)| Chunk::new(*x1 ^ *y1));

        let z2 = x2.value_chunk().zip(y2.value_chunk())
                .map(|(x2, y2)| Chunk::new(*x2 ^ *y2));

        let z3 = x3.value_chunk().zip(y3.value_chunk())
                .map(|(x3, y3)| Chunk::new(*x3 ^ *y3));


        //TOOD add simple iteration through chunks
        layouter.assign_region(
            || "compress",
            |mut region| {
                table_chip.add_row(
                    &mut region, 
                    0, 
                    x0.value_chunk(), 
                    y0.value_chunk(), 
                    z0
                )?;
                Ok(())
            },
        )?;

        println!("{:?}, {:?}, {:?}, {:?}", z0, z1, z2, z3);

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use halo2::{
        dev::MockProver,
    };
    use super::{Circuit};

    use crate::word::{Word};

    #[test]
    fn main_circuit() {
        let circuit = Circuit {
            x: Some(Word::new(0b10101010)),
            y: Some(Word::new(0b01010101))
        };
        let k = 17;
        let prover = MockProver::run(k, &circuit, vec![]).unwrap();
        assert_eq!(prover.verify(), Ok(()));
    }
}