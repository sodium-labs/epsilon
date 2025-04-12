use diesel::backend::Backend;
use diesel::deserialize::{self, FromSql, FromSqlRow};
use diesel::expression::AsExpression;
use diesel::serialize::{self, Output, ToSql};
use diesel::sql_types::Integer;

// VoteType //

#[repr(i32)]
#[derive(Debug, Clone, Copy, FromSqlRow, AsExpression)]
#[diesel(sql_type = Integer)]
pub enum VoteType {
    Like = 1,
    Dislike = 2,
}

impl TryFrom<i32> for VoteType {
    type Error = String;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(VoteType::Like),
            2 => Ok(VoteType::Dislike),
            x => Err(format!("Unrecognized VoteType variant {}", x)),
        }
    }
}

impl<DB> FromSql<Integer, DB> for VoteType
where
    DB: Backend,
    i32: FromSql<Integer, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
        match i32::from_sql(bytes)? {
            1 => Ok(VoteType::Like),
            2 => Ok(VoteType::Dislike),
            x => Err(format!("Unrecognized VoteType variant {}", x).into()),
        }
    }
}

impl<DB> ToSql<Integer, DB> for VoteType
where
    DB: Backend,
    i32: ToSql<Integer, DB>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        match self {
            VoteType::Like => 1.to_sql(out),
            VoteType::Dislike => 2.to_sql(out),
        }
    }
}

// StatisticType //

#[repr(i32)]
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, FromSqlRow, AsExpression)]
#[diesel(sql_type = Integer)]
pub enum StatisticType {
    CrawledPageCount = 1,
    IndexedPageCount = 2,
    ApiRequestCount = 3,
    UserSearchCount = 4,
    DatabaseSize = 5,
    MemoryUsage = 6,
    CpuUsage = 7,
    QueueSize = 8,
    WordCount = 9,
    IndexesCount = 10,
    FaviconsCount = 11,
}

impl<DB> FromSql<Integer, DB> for StatisticType
where
    DB: Backend,
    i32: FromSql<Integer, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
        match i32::from_sql(bytes)? {
            1 => Ok(StatisticType::CrawledPageCount),
            2 => Ok(StatisticType::IndexedPageCount),
            3 => Ok(StatisticType::ApiRequestCount),
            4 => Ok(StatisticType::UserSearchCount),
            5 => Ok(StatisticType::DatabaseSize),
            6 => Ok(StatisticType::MemoryUsage),
            7 => Ok(StatisticType::CpuUsage),
            8 => Ok(StatisticType::QueueSize),
            9 => Ok(StatisticType::WordCount),
            10 => Ok(StatisticType::IndexesCount),
            11 => Ok(StatisticType::FaviconsCount),
            x => Err(format!("Unrecognized StatisticType variant {}", x).into()),
        }
    }
}

impl<DB> ToSql<Integer, DB> for StatisticType
where
    DB: Backend,
    i32: ToSql<Integer, DB>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        match self {
            StatisticType::CrawledPageCount => 1.to_sql(out),
            StatisticType::IndexedPageCount => 2.to_sql(out),
            StatisticType::ApiRequestCount => 3.to_sql(out),
            StatisticType::UserSearchCount => 4.to_sql(out),
            StatisticType::DatabaseSize => 5.to_sql(out),
            StatisticType::MemoryUsage => 6.to_sql(out),
            StatisticType::CpuUsage => 7.to_sql(out),
            StatisticType::QueueSize => 8.to_sql(out),
            StatisticType::WordCount => 9.to_sql(out),
            StatisticType::IndexesCount => 10.to_sql(out),
            StatisticType::FaviconsCount => 11.to_sql(out),
        }
    }
}
