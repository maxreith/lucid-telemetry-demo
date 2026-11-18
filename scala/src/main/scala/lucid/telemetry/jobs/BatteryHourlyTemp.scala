package lucid.telemetry.jobs

import lucid.telemetry.{Events, TelemetryJob}
import org.apache.spark.sql.DataFrame
import org.apache.spark.sql.functions._

object BatteryHourlyTemp extends TelemetryJob {
  val name = "battery_hourly_temp"
  val description = "Average and maximum cell temperature per vehicle per UTC hour."

  def run(events: DataFrame): DataFrame = {
    Events.ofType(Events.dedup(events), "battery_status")
      .groupBy(col("vin"), Events.hourStart(col("event_ts")).as("hour_start_ts"))
      .agg(avg("payload.cell_temp_c").as("avg_temp_c"), max("payload.cell_temp_c").as("max_temp_c"),
        count(lit(1)).as("samples"))
  }
}
