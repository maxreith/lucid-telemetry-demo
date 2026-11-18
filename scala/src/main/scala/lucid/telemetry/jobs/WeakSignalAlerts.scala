package lucid.telemetry.jobs

import lucid.telemetry.{Events, TelemetryJob}
import org.apache.spark.sql.DataFrame
import org.apache.spark.sql.functions._

object WeakSignalAlerts extends TelemetryJob {
  val name = "weak_signal_alerts"
  val description = "Connectivity samples with signal below -110 dBm."

  def run(events: DataFrame): DataFrame = {
    Events.ofType(Events.dedup(events), "connectivity")
      .filter(col("payload.signal_dbm") < -110.0)
      .select(col("vin"), col("event_id"), col("event_ts"), col("payload.signal_dbm").as("signal_dbm"),
        col("payload.network").as("network"))
  }
}
