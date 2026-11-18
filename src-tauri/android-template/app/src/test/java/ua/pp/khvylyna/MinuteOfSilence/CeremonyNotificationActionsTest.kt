package ua.pp.khvylyna.MinuteOfSilence

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/** Verifies forwarding without storing a second copy of the skip setting. */
class CeremonyNotificationActionsTest {
  @Test
  fun actionsAreAvailableOnlyWhileTheBackendIsAttached() {
    val callback: (String, String) -> Unit = { _, _ -> }
    CeremonyNotificationActions.attach(callback)
    try {
      assertTrue(CeremonyNotificationActions.isAvailable())
    } finally {
      CeremonyNotificationActions.detach(callback)
    }
    assertFalse(CeremonyNotificationActions.isAvailable())
  }

  @Test
  fun forwardsTheActionAndItsDisplayedDate() {
    val received = mutableListOf<Pair<String, String>>()
    val callback: (String, String) -> Unit = { action, date -> received.add(action to date) }
    CeremonyNotificationActions.attach(callback)
    try {
      assertTrue(CeremonyNotificationActions.dispatch("skip", "2026-11-18"))
      assertTrue(CeremonyNotificationActions.dispatch("restore", "2026-11-18"))
      assertEquals(listOf("skip" to "2026-11-18", "restore" to "2026-11-18"), received)
      assertFalse(CeremonyNotificationActions.dispatch("toggle", "2026-11-18"))
      assertEquals(2, received.size)
    } finally {
      CeremonyNotificationActions.detach(callback)
    }
  }

  @Test
  fun rejectsActionsAfterTheHandlerIsDetached() {
    val callback: (String, String) -> Unit = { _, _ -> error("detached handler called") }
    CeremonyNotificationActions.attach(callback)
    CeremonyNotificationActions.detach(callback)
    assertFalse(CeremonyNotificationActions.dispatch("skip", "2026-11-18"))
    assertFalse(CeremonyNotificationActions.dispatch("restore", "2026-11-18"))
  }

  @Test
  fun detachingThePreviousHandlerDoesNotDetachTheNewOne() {
    val previous: (String, String) -> Unit = { _, _ -> error("old handler called") }
    val received = mutableListOf<String>()
    val current: (String, String) -> Unit = { action, _ -> received.add(action) }
    CeremonyNotificationActions.attach(previous)
    CeremonyNotificationActions.attach(current)
    try {
      CeremonyNotificationActions.detach(previous)
      assertTrue(CeremonyNotificationActions.dispatch("restore", "2026-11-18"))
      assertEquals(listOf("restore"), received)
    } finally {
      CeremonyNotificationActions.detach(current)
    }
  }
}
