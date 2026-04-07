# Copyright Sebastian Wiesner <sebastian@swsnr.de>
#
# Licensed under the EUPL
#
# See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

"""Various utilities."""

import asyncio
from collections.abc import Callable

from gi.repository import Gio, GObject

type AsyncFinish[T] = Callable[[Gio.AsyncResult], T]
type AsyncCallback = Callable[[GObject.Object, Gio.AsyncResult], None]
type AsyncBegin[T] = Callable[[Gio.Cancellable, AsyncCallback], None]


def _async_finish[T](
    f: asyncio.Future[T], finish: AsyncFinish[T], result: Gio.AsyncResult
) -> None:
    if f.cancelled():
        return
    try:
        r = finish(result)
    except BaseException as error:
        f.set_exception(error)
    else:
        f.set_result(r)


# We do async Gio operations ourselves, with traditional Gio callbacks and
# asyncio futures, because the async wrappers in PyGObject have a few annoying
# issues.
#
# - https://gitlab.gnome.org/GNOME/pygobject/-/issues/755 (TypeError when cancelling)
# - https://github.com/pygobject/pygobject-stubs/issues/220 (incorrect types for
#   Awaitable overloads)
#
# As such, gio_async_result currently works better and feels safer to use.


async def gio_async_result[T](
    async_begin: AsyncBegin[T], async_finish: AsyncFinish[T]
) -> T:
    """Obtain an async result from a Gio operation.

    Make a callback-based async operation work as an async function.

    `async_begin` starts the operation, with a Gio cancellable for the operation,
    and the callback to use.

    `async_finish` is invoked to obtain the actual return value from the Gio
    async result.
    """
    cancellable = Gio.Cancellable()
    f: asyncio.Future[T] = asyncio.get_event_loop().create_future()

    def _propagate_cancel(f: asyncio.Future[T]) -> None:
        if f.cancelled():
            cancellable.cancel()

    f.add_done_callback(_propagate_cancel)
    async_begin(cancellable, lambda _, result: _async_finish(f, async_finish, result))
    return await f
