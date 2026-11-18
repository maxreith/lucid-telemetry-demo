package lucid.telemetry.jobs

import lucid.telemetry.{Events, TelemetryJob}
import org.apache.spark.sql.DataFrame
import org.apache.spark.sql.functions._

object OtaFailures extends TelemetryJob {
  val name = "ota_failures"
  val description = "Failed OTA updates with their error code."

  def run(events: DataFrame): DataFrame = {
    Events.ofType(Events.dedup(events), "ota_status")
      .filter(col("payload.state") === "failed")
      .select(col("vin"), col("payload.campaign_id").as("campaign_id"), col("payload.error_code").as("error_code"),
        col("event_ts"))
  }
}
