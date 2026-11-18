package lucid.telemetry.jobs

import lucid.telemetry.{Events, TelemetryJob}
import org.apache.spark.sql.DataFrame
import org.apache.spark.sql.functions._

object ConnectivityDaily extends TelemetryJob {
  val name = "connectivity_daily"
  val description = "Average signal strength and share of 5G samples per vehicle per UTC day."

  def run(events: DataFrame): DataFrame = {
    Events.ofType(Events.dedup(events), "connectivity")
      .groupBy(col("vin"), Events.day(col("event_ts")).as("day"))
      .agg(avg("payload.signal_dbm").as("avg_signal_dbm"),
        avg(when(col("payload.network") === "5G", 1.0).otherwise(0.0)).as("share_5g"),
        count(lit(1)).as("samples"))
  }
}
