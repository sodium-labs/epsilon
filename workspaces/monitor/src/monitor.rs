use database::{
    get_database_size,
    models::NewStatistic,
    schema::{favicons, indexes, pages, queries, queue, statistics, words},
    types::StatisticType,
    DbPool,
};
use diesel::{BoolExpressionMethods, ExpressionMethods, QueryDsl, QueryResult, RunQueryDsl};
use std::{error::Error, sync::Arc, time::Duration};
use sysinfo::{Pid, System};
use tokio::{sync::Mutex, time::sleep};
use utils::sql::get_sql_timestamp;

pub const MAX_ANALYTICS_AGE: i64 = 86_400_000 * 3;

pub const MAX_SYSTEM_ANALYTICS_AGE: i64 = 86_400_000;

/// Monitor the process and save analytics
pub struct Monitor {
    db_pool: DbPool,
    system: System,
    current_pid: Pid,
}

impl Monitor {
    pub fn new(db_pool: DbPool) -> Self {
        let pid = sysinfo::get_current_pid().expect("Failed to get the current PID");

        Self {
            db_pool,
            system: System::new(),
            current_pid: pid,
        }
    }

    pub async fn run(monitor: Monitor) {
        let monitor = Arc::new(Mutex::new(monitor));

        // Run the system analytics each 60s
        let monitor_clone = monitor.clone();
        let t1 = tokio::spawn(async move {
            loop {
                sleep(Duration::from_secs(60)).await;
                {
                    let guard = &mut monitor_clone.lock().await;
                    if let Err(e) = guard.save_sys_analytics() {
                        eprintln!("[Monitor] Failed to monitor system: {e}");
                    }
                }
            }
        });

        // Run the database analytics at start after 60s and every 10min
        let monitor_clone = monitor.clone();
        let t2 = tokio::spawn(async move {
            sleep(Duration::from_secs(60)).await;

            loop {
                {
                    let guard = monitor_clone.lock().await;
                    if let Err(e) = guard.save_db_analytics() {
                        eprintln!("[Monitor] Failed to monitor database: {e}");
                    }
                }
                sleep(Duration::from_secs(600)).await;
            }
        });

        // Delete the old analytics at start after 60s and every hour
        let monitor_clone = monitor.clone();
        let t3 = tokio::spawn(async move {
            sleep(Duration::from_secs(60)).await;

            loop {
                {
                    let guard = monitor_clone.lock().await;
                    if let Err(e) = guard.delete_old_analytics() {
                        eprintln!("[Monitor] Failed to delete old analytics: {e}");
                    }
                }
                sleep(Duration::from_secs(3_600)).await;
            }
        });

        let _ = tokio::join!(t1, t2, t3);
    }

    fn save_sys_analytics(&mut self) -> QueryResult<()> {
        if let Some(process) = self.system.process(self.current_pid) {
            let now = get_sql_timestamp();

            let new_statistics = vec![
                NewStatistic {
                    timestamp: now,
                    statistic_type: database::types::StatisticType::CpuUsage,
                    value: (process.cpu_usage() * 10000.0) as i64,
                },
                NewStatistic {
                    timestamp: now,
                    statistic_type: database::types::StatisticType::MemoryUsage,
                    value: process.memory() as i64,
                },
            ];

            diesel::insert_into(statistics::table)
                .values(new_statistics)
                .execute(&mut self.db_pool.get().unwrap())?;
        } else {
            eprintln!("[Monitor] Failed to get the current process infos");
        }

        Ok(())
    }

    fn save_db_analytics(&self) -> Result<(), Box<dyn Error>> {
        let conn = &mut self.db_pool.get().unwrap();

        let now = get_sql_timestamp();

        let new_statistics = vec![
            NewStatistic {
                timestamp: now,
                statistic_type: StatisticType::CrawledPageCount,
                value: pages::table.count().get_result::<i64>(conn)?,
            },
            NewStatistic {
                timestamp: now,
                statistic_type: StatisticType::IndexedPageCount,
                value: pages::table
                    .filter(pages::last_indexed.is_not_null())
                    .count()
                    .get_result::<i64>(conn)?,
            },
            NewStatistic {
                timestamp: now,
                statistic_type: StatisticType::DatabaseSize,
                value: get_database_size(conn)?,
            },
            NewStatistic {
                timestamp: now,
                statistic_type: StatisticType::UserSearchCount,
                value: queries::table.count().get_result::<i64>(conn)?,
            },
            NewStatistic {
                timestamp: now,
                statistic_type: StatisticType::QueueSize,
                value: queue::table.count().get_result::<i64>(conn)?,
            },
            NewStatistic {
                timestamp: now,
                statistic_type: StatisticType::WordCount,
                value: words::table.count().get_result::<i64>(conn)?,
            },
            NewStatistic {
                timestamp: now,
                statistic_type: StatisticType::IndexesCount,
                value: indexes::table.count().get_result::<i64>(conn)?,
            },
            NewStatistic {
                timestamp: now,
                statistic_type: StatisticType::FaviconsCount,
                value: favicons::table.count().get_result::<i64>(conn)?,
            },
        ];

        diesel::insert_into(statistics::table)
            .values(new_statistics)
            .execute(conn)?;

        Ok(())
    }

    fn delete_old_analytics(&self) -> QueryResult<()> {
        let now = get_sql_timestamp();
        let conn = &mut self.db_pool.get().unwrap();

        diesel::delete(statistics::table)
            .filter(
                statistics::statistic_type
                    .eq_any(vec![StatisticType::CpuUsage, StatisticType::MemoryUsage])
                    .and(statistics::timestamp.le(now - MAX_SYSTEM_ANALYTICS_AGE)),
            )
            .execute(conn)?;

        diesel::delete(statistics::table)
            .filter(statistics::timestamp.le(now - MAX_ANALYTICS_AGE))
            .execute(conn)?;

        Ok(())
    }
}
