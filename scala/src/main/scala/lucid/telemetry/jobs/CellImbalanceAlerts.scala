package lucid.telemetry.jobs

import lucid.telemetry.{Events, TelemetryJob}
import org.apache.spark.sql.DataFrame
import org.apache.spark.sql.functions._

object CellImbalanceAlerts extends TelemetryJob {
  val name = "cell_imbalance_alerts"
  val description = "Battery readings where the cell voltage spread exceeds 0.08 V."

  def run(events: DataFrame): DataFrame = {
    Events.ofType(Events.dedup(events), "battery_status")
      .withColumn("imbalance_v", col("payload.cell_voltage_max") - col("payload.cell_voltage_min"))
      .filter(col("imbalance_v") > 0.08)
      .select("vin", "event_id", "event_ts", "imbalance_v")
  }
}
