package lucid.telemetry.jobs

import lucid.telemetry.{Events, TelemetryJob}
import org.apache.spark.sql.DataFrame
import org.apache.spark.sql.functions._

object OtaInstallDuration extends TelemetryJob {
  val name = "ota_install_duration"
  val description = "Seconds from install start to install success per vehicle and campaign."

  def run(events: DataFrame): DataFrame = {
    Events.ofType(Events.dedup(events), "ota_status")
      .groupBy("vin", "payload.campaign_id")
      .agg(
        min(when(col("payload.state") === "installing", col("event_ts"))).as("installing_ts"),
        min(when(col("payload.state") === "installed", col("event_ts"))).as("installed_ts")
      )
      .filter(col("installing_ts").isNotNull && col("installed_ts").isNotNull)
      .withColumn("duration_s", (col("installed_ts") - col("installing_ts")) / 1000.0)
      .select("vin", "campaign_id", "duration_s")
  }
}
