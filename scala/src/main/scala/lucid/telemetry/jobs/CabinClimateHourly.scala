package lucid.telemetry.jobs

import lucid.telemetry.{Events, TelemetryJob}
import org.apache.spark.sql.DataFrame
import org.apache.spark.sql.functions._

object CabinClimateHourly extends TelemetryJob {
  val name = "cabin_climate_hourly"
  val description = "Average cabin and outside temperature per vehicle per UTC hour."

  def run(events: DataFrame): DataFrame = {
    Events.ofType(Events.dedup(events), "climate")
      .groupBy(col("vin"), Events.hourStart(col("event_ts")).as("hour_start_ts"))
      .agg(avg("payload.cabin_temp_c").as("avg_cabin_temp_c"), avg("payload.outside_temp_c").as("avg_outside_temp_c"))
  }
}
