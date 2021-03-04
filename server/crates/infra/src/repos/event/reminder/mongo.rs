use super::IReminderRepo;
use crate::repos::shared::mongo_repo;
use crate::repos::shared::repo::DeleteResult;
use mongo_repo::MongoDocument;
use mongodb::{
    bson::doc,
    bson::{oid::ObjectId, Document},
    Collection, Database,
};
use nettu_scheduler_domain::Reminder;
use serde::{Deserialize, Serialize};

pub struct MongoReminderRepo {
    collection: Collection,
}

impl MongoReminderRepo {
    pub fn new(db: &Database) -> Self {
        Self {
            collection: db.collection("calendar-event-reminders"),
        }
    }
}

#[async_trait::async_trait]
impl IReminderRepo for MongoReminderRepo {
    async fn bulk_insert(&self, reminders: &[Reminder]) -> anyhow::Result<()> {
        mongo_repo::bulk_insert::<_, ReminderMongo>(&self.collection, reminders).await
    }

    async fn find_by_event_and_priority(&self, event_id: &str, priority: i64) -> Option<Reminder> {
        let filter = doc! {
            "event_id": event_id,
            "priority": priority,
        };

        mongo_repo::find_one_by::<_, ReminderMongo>(&self.collection, filter.clone()).await
    }

    async fn delete_all_before(&self, before_inc: i64) -> Vec<Reminder> {
        let filter = doc! {
            "remind_at": {
                "$lte": before_inc
            }
        };

        // Find before deleting
        let docs =
            match mongo_repo::find_many_by::<_, ReminderMongo>(&self.collection, filter.clone())
                .await
            {
                Ok(docs) => docs,
                Err(err) => {
                    println!("Error: {:?}", err);
                    return vec![];
                }
            };

        // Now delete
        if let Err(err) = self.collection.delete_many(filter, None).await {
            println!("Error: {:?}", err);
        }

        docs
    }

    async fn delete_by_events(&self, event_ids: &[String]) -> anyhow::Result<DeleteResult> {
        let filter = doc! {
            "event_id": {
                "$in": event_ids
            }
        };
        self.collection
            .delete_many(filter, None)
            .await
            .map(|res| DeleteResult {
                deleted_count: res.deleted_count,
            })
            .map_err(anyhow::Error::new)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct ReminderMongo {
    _id: ObjectId,
    remind_at: i64,
    event_id: String,
    account_id: String,
    priority: i64,
}

impl MongoDocument<Reminder> for ReminderMongo {
    fn to_domain(&self) -> Reminder {
        Reminder {
            id: self._id.to_string(),
            remind_at: self.remind_at,
            event_id: self.event_id.clone(),
            account_id: self.account_id.clone(),
            priority: self.priority,
        }
    }

    fn from_domain(event: &Reminder) -> Self {
        Self {
            _id: ObjectId::with_string(&event.id).unwrap(),
            event_id: event.event_id.to_owned(),
            account_id: event.account_id.to_owned(),
            remind_at: event.remind_at,
            priority: event.priority,
        }
    }

    fn get_id_filter(&self) -> Document {
        doc! {
            "_id": self._id.clone()
        }
    }
}