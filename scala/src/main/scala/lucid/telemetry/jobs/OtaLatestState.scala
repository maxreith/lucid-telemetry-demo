package lucid.telemetry.jobs

import lucid.telemetry.{Events, TelemetryJob}
import org.apache.spark.sql.DataFrame
import org.apache.spark.sql.functions._

object OtaLatestState extends TelemetryJob {
  val name = "ota_latest_state"
  val description = "Latest OTA update state per vehicle and campaign, by event time."

  def run(events: DataFrame): DataFrame = {
    Events.latestBy(Events.ofType(Events.dedup(events), "ota_status"), "vin", "payload.campaign_id")
      .select(col("vin"), col("payload.campaign_id").as("campaign_id"), col("payload.state").as("state"),
        col("payload.version").as("version"), col("event_ts"))
  }
}
