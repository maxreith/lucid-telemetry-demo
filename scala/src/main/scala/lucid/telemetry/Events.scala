package lucid.telemetry

import org.apache.spark.sql.{Column, DataFrame, SparkSession}
import org.apache.spark.sql.expressions.Window
import org.apache.spark.sql.functions._
import org.apache.spark.sql.types._

object Events {
  val payloadSchema: StructType = StructType(
    Seq(
      "session_id", "charger_type", "code", "severity", "ecu", "campaign_id", "version", "state",
      "error_code", "trip_id", "network", "kind"
    ).map(StructField(_, StringType)) ++ Seq(
      "soc", "pack_capacity_kwh", "kwh_delta", "power_kw", "cell_temp_c", "cell_voltage_min",
      "cell_voltage_max", "soh_pct", "odometer_km", "lat", "lon", "speed_kph", "fl", "fr", "rl", "rr",
      "cabin_temp_c", "outside_temp_c", "hvac_power_kw", "signal_dbm"
    ).map(StructField(_, DoubleType)) :+ StructField("heading", LongType)
  )

  val schema: StructType = StructType(
    Seq(
      StructField("event_id", StringType),
      StructField("vin", StringType),
      StructField("type", StringType),
      StructField("event_ts", LongType),
      StructField("received_ts", LongType),
      StructField("fw_version", StringType),
      StructField("payload", payloadSchema)
    )
  )

  def load(spark: SparkSession, path: String): DataFrame = spark.read.schema(schema).json(path)

  /** Cars resend events when an upload is not acknowledged; a resent copy keeps its event_id. */
  def dedup(events: DataFrame): DataFrame = events.dropDuplicates("event_id")

  def ofType(events: DataFrame, types: String*): DataFrame = events.filter(col("type").isin(types: _*))

  /** UTC calendar day of an epoch-millis column, as yyyy-MM-dd. */
  def day(ts: Column): Column = date_format(timestamp_millis(ts), "yyyy-MM-dd")

  /** Start of the UTC hour of an epoch-millis column, in epoch millis. */
  def hourStart(ts: Column): Column = (floor(ts / 3600000L) * 3600000L).cast(LongType)

  /** Keeps the most recent event per key by event time; ties are broken by the larger event_id. */
  def latestBy(events: DataFrame, keys: String*): DataFrame = {
    val w = Window.partitionBy(keys.map(col): _*).orderBy(col("event_ts").desc, col("event_id").desc)
    events.withColumn("_rank", row_number().over(w)).filter(col("_rank") === 1).drop("_rank")
  }
}
