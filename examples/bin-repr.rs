use bin2::Reader;
use bin2::Writer;
use bin2::repr::BinRepr;

#[derive(Debug)]
struct Item {
    name: String,
    count: u32,
}

#[derive(Debug)]
struct Player {
    name: String,
    pets: Vec<String>,
    inventory: Vec<Item>,
    health: u32,
    level: u32,
}

impl BinRepr for Item {
    fn write_to<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<()> {
        self.name.write_to(writer)?;
        self.count.write_to(writer)?;
        Ok(())
    }

    fn read_from(reader: &mut Reader) -> std::io::Result<Self> {
        let name = String::read_from(reader)?;
        let count = u32::read_from(reader)?;

        Ok(Self {
            name,
            count
        })
    }
}

fn main() -> std::io::Result<()> {
    let mut writer = Writer::new();
    {
        let player = Player {
            name: "Simon".to_string(),
            pets: vec!["Foo".to_string(), "Bar".to_string()],
            inventory: vec![
                Item {
                    name: "Diamond Pickaxe".to_string(),
                    count: 500,
                },
                Item {
                    name: "123".to_string(),
                    count: 5,
                },
            ],
            health: 100,
            level: 5,
        };

        writer.write(&player.name)?;
        writer.write(&player.pets)?;
        writer.write(&player.inventory)?;
        writer.write(&player.health)?;
        writer.write(&player.level)?;
    }
    
    let mut file = std::fs::File::create("Player.bin")?;
    writer.write_to(&mut file)?;

    let bytes: Vec<u8> = std::fs::read("Player.bin")?;
    let mut reader = Reader::from_bytes(&bytes);
    {
        let player = Player {
            name: reader.read()?,
            pets: reader.read()?,
            inventory: reader.read()?,
            health: reader.read()?,
            level: reader.read()?,
        };

        println!("{:?}", player);
    }

    Ok(())
}
