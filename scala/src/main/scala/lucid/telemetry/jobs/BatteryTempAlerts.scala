package lucid.telemetry.jobs

import lucid.telemetry.{Events, TelemetryJob}
import org.apache.spark.sql.DataFrame
import org.apache.spark.sql.functions._

object BatteryTempAlerts extends TelemetryJob {
  val name = "battery_temp_alerts"
  val description = "Battery readings with a cell temperature above 45 C."

  def run(events: DataFrame): DataFrame = {
    Events.ofType(Events.dedup(events), "battery_status")
      .filter(col("payload.cell_temp_c") > 45.0)
      .select(col("vin"), col("event_id"), col("event_ts"), col("payload.cell_temp_c").as("cell_temp_c"))
  }
}
