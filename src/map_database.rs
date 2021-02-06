#[derive(Debug, thiserror::Error)]
pub enum DBError {
	#[error("database operation failed")]
	DatabaseError,
	#[error("database is not a valid map database")]
	InvalidDatabase,
	#[error("requested data was not found")]
	MissingData,
}

impl From<sqlite::Error> for DBError {
	fn from(_: sqlite::Error) -> Self {
		Self::DatabaseError
	}
}


fn verify_database(conn: &sqlite::Connection) -> Result<(), DBError> {
	let my_assert = |res: bool| -> Result<(), DBError> {
		match res {
			true => Ok(()),
			false => Err(DBError::InvalidDatabase)
		}
	};

	let mut stmt = conn.prepare("PRAGMA table_info(blocks)")?;

	stmt.next()?;
	my_assert(stmt.read::<String>(1)? == "pos")?;
	my_assert(stmt.read::<String>(2)? == "INT")?;
	my_assert(stmt.read::<i64>(5)? == 1)?;
	stmt.next()?;
	my_assert(stmt.read::<String>(1)? == "data")?;
	my_assert(stmt.read::<String>(2)? == "BLOB")?;
	my_assert(stmt.read::<i64>(5)? == 0)?;

	Ok(())
}


pub struct MapDatabaseRows<'a> {
	stmt_get: sqlite::Statement<'a>
}

impl Iterator for MapDatabaseRows<'_> {
	type Item = (i64, Vec<u8>);

	fn next(&mut self) -> Option<Self::Item> {
		match self.stmt_get.next().unwrap() {
			sqlite::State::Row => {
				Some((
					self.stmt_get.read(0).unwrap(),
					self.stmt_get.read(1).unwrap()
				))
			},
			sqlite::State::Done => None
		}
	}
}


pub struct MapDatabase<'a> {
	conn: &'a sqlite::Connection,
	stmt_get: sqlite::Statement<'a>,
	stmt_set: sqlite::Statement<'a>,
	stmt_del: sqlite::Statement<'a>,
	in_transaction: bool,
}

impl<'a> MapDatabase<'a> {
	pub fn new(conn: &'a sqlite::Connection) -> Result<Self, DBError> {
		conn.execute("BEGIN")?;
		verify_database(conn)?;

		let stmt_get = conn.prepare("SELECT data FROM blocks WHERE pos = ?")?;
		let stmt_set = conn.prepare(
			"INSERT OR REPLACE INTO blocks (pos, data) VALUES (?, ?)")?;
		let stmt_del = conn.prepare("DELETE FROM blocks WHERE pos = ?")?;

		Ok(Self {conn, stmt_get, stmt_set, stmt_del, in_transaction: true})
	}

	pub fn is_in_transaction(&self) -> bool {
		self.in_transaction
	}

	#[inline]
	fn begin_if_needed(&self) -> Result<(), DBError> {
		if !self.in_transaction {
			self.conn.execute("BEGIN")?;
		}
		Ok(())
	}

	pub fn commit_if_needed(&mut self) -> Result<(), DBError> {
		if self.in_transaction {
			self.conn.execute("COMMIT")?;
			self.in_transaction = false;
		}
		Ok(())
	}

	pub fn iter_rows(&self) -> MapDatabaseRows {
		self.begin_if_needed().unwrap();
		let stmt = self.conn.prepare("SELECT pos, data FROM blocks").unwrap();
		MapDatabaseRows {stmt_get: stmt}
	}

	pub fn get_block(&mut self, map_key: i64) -> Result<Vec<u8>, DBError> {
		self.begin_if_needed()?;
		self.stmt_get.bind(1, map_key)?;

		let value = match self.stmt_get.next()? {
			sqlite::State::Row => Ok(self.stmt_get.read(0)?),
			sqlite::State::Done => Err(DBError::MissingData)
		};

		self.stmt_get.reset()?;
		value
	}

	pub fn set_block(&mut self, map_key: i64, data: &[u8])
		-> Result<(), DBError>
	{
		self.begin_if_needed()?;
		self.stmt_set.bind(1, map_key)?;
		self.stmt_set.bind(2, data)?;
		self.stmt_set.next()?;
		self.stmt_set.reset()?;
		Ok(())
	}

	pub fn delete_block(&mut self, map_key: i64) -> Result<(), DBError> {
		self.begin_if_needed()?;
		self.stmt_del.bind(1, map_key)?;
		self.stmt_del.next()?;
		self.stmt_del.reset()?;
		Ok(())
	}

	pub fn vacuum(&mut self) -> Result<(), DBError> {
		self.commit_if_needed()?;
		self.conn.execute("VACUUM")?;
		Ok(())
	}
}
