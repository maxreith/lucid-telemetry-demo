package lucid.telemetry.jobs

import lucid.telemetry.{Events, TelemetryJob}
import org.apache.spark.sql.DataFrame
import org.apache.spark.sql.functions._

object BatterySohLatest extends TelemetryJob {
  val name = "battery_soh_latest"
  val description = "Latest reported battery state of health per vehicle."

  def run(events: DataFrame): DataFrame = {
    Events.latestBy(Events.ofType(Events.dedup(events), "battery_status"), "vin")
      .select(col("vin"), col("payload.soh_pct").as("soh_pct"), col("event_ts"))
  }
}
