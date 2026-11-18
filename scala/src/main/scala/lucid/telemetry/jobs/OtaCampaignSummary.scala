package lucid.telemetry.jobs

import lucid.telemetry.{Events, TelemetryJob}
import org.apache.spark.sql.DataFrame
import org.apache.spark.sql.functions._

object OtaCampaignSummary extends TelemetryJob {
  val name = "ota_campaign_summary"
  val description = "Vehicles per OTA campaign by their latest state."

  def run(events: DataFrame): DataFrame = {
    val latest = Events.latestBy(Events.ofType(Events.dedup(events), "ota_status"), "vin", "payload.campaign_id")
    latest
      .groupBy("payload.campaign_id")
      .agg(
        count(lit(1)).as("vehicles"),
        sum(when(col("payload.state") === "installed", 1).otherwise(0)).as("installed"),
        sum(when(col("payload.state") === "failed", 1).otherwise(0)).as("failed"),
        sum(when(col("payload.state").isin("downloading", "installing"), 1).otherwise(0)).as("in_progress")
      )
  }
}
