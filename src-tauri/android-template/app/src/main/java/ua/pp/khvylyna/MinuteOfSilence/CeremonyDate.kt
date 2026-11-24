package ua.pp.khvylyna.MinuteOfSilence

import java.util.GregorianCalendar
import java.util.Locale
import java.util.TimeZone

/** Formats ceremony dates with the Gregorian calendar used by the settings contract. */
internal object CeremonyDate {
  fun isoDate(epochMillis: Long, zone: TimeZone): String {
    val date = GregorianCalendar(zone).apply { timeInMillis = epochMillis }
    return String.format(
      Locale.ROOT,
      "%04d-%02d-%02d",
      date.get(GregorianCalendar.YEAR),
      date.get(GregorianCalendar.MONTH) + 1,
      date.get(GregorianCalendar.DAY_OF_MONTH)
    )
  }
}
