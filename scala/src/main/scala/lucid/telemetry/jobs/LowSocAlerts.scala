package lucid.telemetry.jobs

import lucid.telemetry.{Events, TelemetryJob}
import org.apache.spark.sql.DataFrame
import org.apache.spark.sql.functions._

object LowSocAlerts extends TelemetryJob {
  val name = "low_soc_alerts"
  val description = "Battery readings with state of charge below 15%."

  def run(events: DataFrame): DataFrame = {
    Events.ofType(Events.dedup(events), "battery_status")
      .filter(col("payload.soc") < 15.0)
      .select(col("vin"), col("event_id"), col("event_ts"), col("payload.soc").as("soc"))
  }
}
